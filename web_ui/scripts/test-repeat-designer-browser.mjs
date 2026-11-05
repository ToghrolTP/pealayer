import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
// Use an existing tool installation; do not download browsers during tests.
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE_PATH).href);
const browser=await chromium.launch({channel:'msedge',headless:true});
try {
  const page=await browser.newPage();
  const errors=[]; page.on('pageerror',error=>errors.push(error.message));
  const read=async()=>JSON.parse(await page.locator('#fixture-state').textContent());
  for(const width of [390,768,1440]) {
    await page.setViewportSize({width,height:850});
    await page.goto('http://127.0.0.1:18191/scripts/repeat-designer-fixture.html');
    await page.locator('.sequence-authoring__cue').first().waitFor();
    await page.getByRole('checkbox',{name:'Show repeat ghosts'}).check();
    assert.ok(await page.locator('.is-effect-repeat').count()>0);
    assert.equal(await page.locator('.is-effect-repeat').first().evaluate(el=>getComputedStyle(el).borderTopStyle),'dashed');
    assert.equal(await page.locator('.is-repeat:not(.is-effect-repeat)').first().evaluate(el=>getComputedStyle(el).borderTopStyle),'dotted');
    const before=await read();
    await page.locator('[data-step-index="1"][data-repeat-index="0"][data-effect-repeat-index="0"]').click();
    const ghost=page.locator('[data-step-index="0"][data-repeat-index="0"][data-effect-repeat-index="1"]');
    await ghost.click();
    assert.equal(await page.locator('[data-step-index="0"][aria-pressed="false"]').count(),0);
    assert.equal(await ghost.evaluate(el=>getComputedStyle(el).outlineStyle),'dashed','selected whole-effect ghosts retain their dashed outline');
    assert.deepEqual((await read()).steps,before.steps);
    const stepGhost=page.locator('[data-step-index="0"][data-repeat-index="1"][data-effect-repeat-index="0"]');
    await stepGhost.focus(); await stepGhost.press('ArrowRight');
    assert.equal((await read()).steps[0].repeat_interval_ms,800);
    await stepGhost.press('ArrowLeft');
    assert.deepEqual((await read()).steps,before.steps);
    await ghost.press('ArrowRight');
    assert.equal((await read()).repeat.interval,2050);
    assert.deepEqual((await read()).steps,before.steps);
    await ghost.scrollIntoViewIfNeeded();
    const rect=await ghost.boundingBox();
    await page.mouse.move(rect.x+12,rect.y+rect.height/2); await page.mouse.down();
    await page.mouse.move(rect.x+52,rect.y+rect.height/2,{steps:5}); await page.mouse.up();
    assert.equal((await read()).repeat.interval,2550);
    assert.deepEqual((await read()).steps,before.steps);
    await ghost.click({button:'right'});
    await page.getByRole('spinbutton',{name:'Count',exact:true}).fill('4');
    await page.getByRole('spinbutton',{name:'Count',exact:true}).press('Enter');
    assert.equal((await read()).repeat.count,4);
    await page.keyboard.press('Escape');
    await page.getByRole('checkbox',{name:'Show repeat ghosts'}).uncheck();
    assert.equal(await page.locator('.is-repeat').count(),0);
    assert.equal((await read()).repeat.count,4);
    await page.reload();
    await page.locator('.sequence-authoring__cue').first().waitFor();
    assert.equal(await page.locator('.is-repeat').count(),0,'visibility preference survives remount');
    await page.getByRole('checkbox',{name:'Show repeat ghosts'}).check();
    assert.ok(await page.locator('.is-repeat').count()>0);
    assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),`page overflow at ${width}`);
    if(process.env.PEALAYER_SCREENSHOT_DIR) await page.screenshot({path:`${process.env.PEALAYER_SCREENSHOT_DIR}/repeat-designer-${width}.png`,fullPage:true});
  }
  assert.deepEqual(errors,[]);
  console.log('Browser repeat projections: selection, interval drag/keyboard, count menu, visibility, responsive 390/768/1440 passed.');
} finally { await browser.close(); }
