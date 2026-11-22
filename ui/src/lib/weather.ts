// Météo via Open-Meteo (gratuit, sans clé, sans compte). Uniquement si l'utilisateur l'active :
// la ville saisie et les coordonnées sont envoyées à api.open-meteo.com, rien d'autre.
export interface GeoResult {
  name: string;
  lat: number;
  lon: number;
  country: string;
}

export async function geocode(city: string, lang: string): Promise<GeoResult | null> {
  const u = `https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(city)}&count=1&language=${lang}&format=json`;
  const r = await fetch(u);
  if (!r.ok) return null;
  const j = await r.json();
  const g = j.results?.[0];
  return g ? { name: g.name, lat: g.latitude, lon: g.longitude, country: g.country ?? '' } : null;
}

export interface Current {
  temp: number;
  kind: 'sun' | 'partly' | 'cloud' | 'fog' | 'rain' | 'snow' | 'storm';
}

function kindOf(code: number): Current['kind'] {
  if (code === 0) return 'sun';
  if (code <= 2) return 'partly';
  if (code === 3) return 'cloud';
  if (code === 45 || code === 48) return 'fog';
  if (code >= 95) return 'storm';
  if ((code >= 71 && code <= 77) || code === 85 || code === 86) return 'snow';
  if (code >= 51) return 'rain';
  return 'cloud';
}

export async function currentWeather(lat: number, lon: number, unit: 'c' | 'f'): Promise<Current | null> {
  const u = `https://api.open-meteo.com/v1/forecast?latitude=${lat}&longitude=${lon}&current=temperature_2m,weather_code${unit === 'f' ? '&temperature_unit=fahrenheit' : ''}`;
  const r = await fetch(u);
  if (!r.ok) return null;
  const j = await r.json();
  const c = j.current;
  return c ? { temp: Math.round(c.temperature_2m), kind: kindOf(c.weather_code) } : null;
}
