import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
// Use an existing tool installation; do not download browsers during tests.
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE_PATH).href);
const browser=await chromium.launch({channel:'msedge',headless:true});
try {
  const page=await browser.newPage();
  const errors=[]; page.on('pageerror',error=>errors.push(error.message));
  const read=async()=>JSON.parse(await page.locator('#fixture-state').textContent());
  for(const light of [false,true]) for(const width of [390,768,1440]) {
    await page.setViewportSize({width,height:850});
    await page.goto(`http://127.0.0.1:18191/scripts/repeat-designer-fixture.html${light?'?light':''}`);
    await page.locator('.sequence-authoring__cue').first().waitFor();
    if(width===390) assert.equal(await page.getByRole('checkbox',{name:'Show repeat ghosts'}).isChecked(),true,'ghosts default to enabled');
    await page.getByRole('checkbox',{name:'Show repeat ghosts'}).check();
    assert.ok(await page.locator('.is-effect-repeat').count()>0);
    assert.equal(await page.locator('.is-effect-repeat').first().evaluate(el=>getComputedStyle(el).borderTopStyle),'dashed');
    assert.equal(await page.locator('.is-repeat:not(.is-effect-repeat)').first().evaluate(el=>getComputedStyle(el).borderTopStyle),'dotted');
    const before=await read();
    await page.locator('[data-step-index="1"][data-repeat-index="0"][data-effect-repeat-index="0"]').click();
    const ghost=page.locator('[data-step-index="0"][data-repeat-index="0"][data-effect-repeat-index="1"]');
    const stepGhost=page.locator('[data-step-index="0"][data-repeat-index="1"][data-effect-repeat-index="0"]');
    await ghost.click();
    assert.equal(await page.locator('[data-step-index="0"][aria-pressed="false"]').count(),0);
    assert.equal(await ghost.evaluate(el=>getComputedStyle(el).outlineStyle),'dashed','selected whole-effect ghosts retain their dashed outline');
    assert.deepEqual((await read()).steps,before.steps);
    for(const target of [stepGhost,ghost]) {
      await target.scrollIntoViewIfNeeded();
      const start=await target.boundingBox(),state=await read();
      await page.mouse.move(start.x+12,start.y+start.height/2);await page.mouse.down();
      await page.mouse.move(start.x+36,start.y+start.height/2,{steps:4});
      await page.keyboard.press('Escape');await page.mouse.up();
      assert.deepEqual(await read(),state,'Escape cancels step/whole-effect preview and release cannot commit it');
    }
    await stepGhost.scrollIntoViewIfNeeded();
    const stepRect=await stepGhost.boundingBox();
    await page.mouse.move(stepRect.x+12,stepRect.y+stepRect.height/2);await page.mouse.down();
    await page.mouse.move(stepRect.x+28,stepRect.y+stepRect.height/2,{steps:4});await page.mouse.up();
    assert.equal((await read()).steps[0].repeat_interval_ms,950,'dragging a step proxy adjusts only its shared interval');
    assert.equal((await read()).steps.length,before.steps.length,'projections never materialize persisted cues');
    assert.equal((await read()).steps[0].at_us,before.steps[0].at_us,'spacing does not move the prime');
    await stepGhost.focus();await stepGhost.press('ArrowLeft');await stepGhost.press('ArrowLeft');await stepGhost.press('ArrowLeft');await stepGhost.press('ArrowLeft');
    assert.deepEqual((await read()).steps,before.steps);
    for(const delta of [8,-8]) {
      await stepGhost.scrollIntoViewIfNeeded();
      const edge=await stepGhost.boundingBox();
      await page.mouse.move(edge.x+edge.width-3,edge.y+edge.height/2);await page.mouse.down();
      await page.mouse.move(edge.x+edge.width-3+delta,edge.y+edge.height/2,{steps:4});await page.mouse.up();
      assert.equal((await read()).steps[0].duration_ms,delta>0?600:500,'a proxy resize changes the one shared source length');
      assert.equal((await read()).steps[0].repeat_interval_ms,750,'length edits do not silently change the repeat interval');
      assert.equal(await page.locator('[data-step-index="0"][aria-pressed="false"]').count(),0,'resized source and all projections retain shared selection');
    }
    assert.deepEqual((await read()).steps,before.steps);
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
    if(process.env.PEALAYER_SCREENSHOT_DIR) await page.screenshot({path:`${process.env.PEALAYER_SCREENSHOT_DIR}/repeat-designer-${width}${light?'-light':''}.png`,fullPage:true});
    await page.locator('[data-step-index="0"][data-repeat-index="0"][data-effect-repeat-index="0"]').click();
    assert.equal(await page.locator('[data-step-index="0"][aria-pressed="false"]').count(),0,'selecting the prime selects every projection too');
    await page.keyboard.press('Delete');
    assert.deepEqual((await read()).steps,[before.steps[1]],'Delete removes the source once, not one saved step per projection');
  }
  assert.deepEqual(errors,[]);
  console.log('Browser repeat projections: shared selection/deletion/length, interval drag/keyboard, Escape cancellation, count menu, saved visibility; light/dark 390/768/1440 passed.');
} finally { await browser.close(); }
