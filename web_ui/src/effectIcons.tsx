import {
  ApiOutlined,
  BellOutlined,
  BulbOutlined,
  CarOutlined,
  CustomerServiceOutlined,
  FireOutlined,
  PlayCircleOutlined,
  PoweroffOutlined,
  RadarChartOutlined,
  SettingOutlined,
  SoundOutlined,
  ThunderboltOutlined,
  WifiOutlined,
} from '@ant-design/icons';

const presets = [
  ['plug', 'Plug'], ['lightning', 'Lightning'], ['lightbulb', 'Light bulb'],
  ['lamp', 'Lamp'], ['fan', 'Fan'], ['power', 'Power'], ['speaker', 'Speaker'],
  ['radio', 'Radio'], ['seat', 'Seat'], ['car', 'Car'], ['door', 'Door'],
  ['bell', 'Bell'], ['fire', 'Fire'], ['snowflake', 'Snowflake'],
  ['thermometer', 'Thermometer'], ['waveform', 'Waveform'],
  ['circuitry', 'Circuitry'], ['gear', 'Gear'],
] as const;

export const effectGlyph = (icon: string) => {
  switch (icon.trim().toLowerCase()) {
    case 'lightning': return <ThunderboltOutlined />;
    case 'lightbulb':
    case 'lamp': return <BulbOutlined />;
    case 'power': return <PoweroffOutlined />;
    case 'speaker': return <SoundOutlined />;
    case 'radio': return <WifiOutlined />;
    case 'car': return <CarOutlined />;
    case 'bell': return <BellOutlined />;
    case 'fire': return <FireOutlined />;
    case 'gear': return <SettingOutlined />;
    case 'circuitry': return <ApiOutlined />;
    case 'seat':
    case 'door': return <CustomerServiceOutlined />;
    case 'waveform':
    case 'fan':
    case 'snowflake':
    case 'thermometer': return <RadarChartOutlined />;
    default: return <PlayCircleOutlined />;
  }
};

export const effectIconOptions = presets.map(([value, label]) => ({
  value,
  label: <span className="effect-icon-option">{effectGlyph(value)}<span>{label}</span></span>,
}));
