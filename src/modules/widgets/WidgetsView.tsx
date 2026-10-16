import { useEffect, useState } from 'react';
import { task, useApp } from '../../app/store';
import { errorMessage } from '../../lib/errors';
import { Icon } from '../../ui/Icon';
import { Loading } from '../../ui/primitives';
import { widgetsApi } from './widgetsApi';
import {
  degrees,
  lastDays,
  localDate,
  newHabit,
  streak,
  toggleDay,
  weatherGlyph,
  weatherWords,
  type Habit,
  type Weather,
} from './widgetLogic';

function WeatherCard() {
  const settings = useApp((s) => s.data!.settings);
  const [weather, setWeather] = useState<Weather | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const enabled = settings.weather_enabled && settings.weather_place.trim() !== '';
  const load = (refresh: boolean) => {
    setBusy(true);
    setError(null);
    widgetsApi
      .weather(refresh)
      .then(setWeather)
      .catch((e) => setError(errorMessage(e)))
      .finally(() => setBusy(false));
  };
  useEffect(() => {
    if (enabled) load(false);
    else setWeather(null);
  }, [enabled, settings.weather_place, settings.weather_fahrenheit]);
  return (
    <article className="widget-card weather-card">
      <header>
        <h4>Weather</h4>
        <span className="spacer" />
        {enabled && (
          <button
            className="icon-button"
            aria-label="Refresh weather"
            title="Refresh weather"
            disabled={busy}
            onClick={() => load(true)}
          >
            <Icon name="refresh" size={14} />
          </button>
        )}
      </header>
      {!settings.weather_enabled ? (
        <p className="widget-hint">
          Off. Turn it on in Settings. It sends your city to open-meteo.com.
        </p>
      ) : !enabled ? (
        <p className="widget-hint">Add your city in Settings to see the weather.</p>
      ) : busy && !weather ? (
        <Loading label="Checking the sky…" inline />
      ) : error ? (
        <p className="inline-error">{error}</p>
      ) : weather ? (
        <div className="weather-now">
          <span className="weather-glyph" aria-hidden="true">
            {weatherGlyph(weather.code, weather.is_day)}
          </span>
          <span className="weather-temp">{degrees(weather.temperature)}</span>
          <span className="weather-detail">
            <strong>{weatherWords(weather.code)}</strong>
            <small>
              {weather.place} · H {degrees(weather.high)} L {degrees(weather.low)}
            </small>
          </span>
        </div>
      ) : null}
    </article>
  );
}

function HabitsCard() {
  const [habits, setHabits] = useState<Habit[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [name, setName] = useState('');
  const today = new Date();
  const days = lastDays(today, 7);
  const todayKey = localDate(today);
  useEffect(() => {
    widgetsApi
      .habits()
      .then(setHabits)
      .catch((e) => setError(errorMessage(e)));
  }, []);
  const save = (next: Habit[]) => {
    const previous = habits;
    setHabits(next);
    void task(() => widgetsApi.saveHabits(next)).then((saved) => {
      if (!saved) setHabits(previous);
    });
  };
  const add = () => {
    if (!habits || !name.trim()) return;
    save([...habits, newHabit(name, crypto.randomUUID())]);
    setName('');
  };
  return (
    <article className="widget-card habits-card">
      <header>
        <h4>Habits</h4>
        <span className="spacer" />
        <span className="habit-days" aria-hidden="true">
          {days.map((d) => (
            <span key={d}>
              {new Date(`${d}T12:00`).toLocaleDateString(undefined, { weekday: 'narrow' })}
            </span>
          ))}
        </span>
      </header>
      {error ? (
        <p className="inline-error">{error}</p>
      ) : !habits ? (
        <Loading label="Loading habits…" inline />
      ) : (
        <>
          {habits.length === 0 && (
            <p className="widget-hint">Add something you want to do daily.</p>
          )}
          <ul className="habit-list">
            {habits.map((habit) => (
              <li key={habit.id}>
                <span className="habit-name" title={habit.name}>
                  {habit.name}
                </span>
                <small className="habit-streak">{streak(habit, today)}d</small>
                <span className="habit-days">
                  {days.map((d) => {
                    const done = habit.done.includes(d);
                    return (
                      <button
                        key={d}
                        type="button"
                        className={`habit-dot ${done ? 'is-done' : ''} ${d === todayKey ? 'is-today' : ''}`}
                        aria-pressed={done}
                        aria-label={`${habit.name} on ${d}`}
                        onClick={() =>
                          save(habits.map((h) => (h.id === habit.id ? toggleDay(h, d) : h)))
                        }
                      />
                    );
                  })}
                </span>
                <button
                  className="icon-button habit-remove"
                  aria-label={`Remove ${habit.name}`}
                  title="Remove"
                  onClick={() => save(habits.filter((h) => h.id !== habit.id))}
                >
                  <Icon name="close" size={12} />
                </button>
              </li>
            ))}
          </ul>
          {habits.length < 12 && (
            <form
              className="habit-add"
              onSubmit={(e) => {
                e.preventDefault();
                add();
              }}
            >
              <input
                value={name}
                maxLength={40}
                placeholder="New habit"
                onChange={(e) => setName(e.target.value)}
              />
              <button type="submit" disabled={!name.trim()}>
                Add
              </button>
            </form>
          )}
        </>
      )}
    </article>
  );
}

/** Small built-in widgets. Habits stay on this computer; weather is opt-in. */
export function WidgetsView() {
  return (
    <section className="feature widgets-view">
      <WeatherCard />
      <HabitsCard />
    </section>
  );
}
