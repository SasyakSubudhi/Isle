import { useEffect, useRef, useState } from 'react';
import { api } from '../../lib/api';
import { task, useApp } from '../../app/store';
import type { DictionaryResult } from '../../lib/types';
import { Empty, ErrorState, IconButton, Loading } from '../../ui/primitives';
import { Icon } from '../../ui/Icon';

export function DictionaryView() {
  const selection = useApp((s) => s.selection);
  const data = useApp((s) => s.data)!;
  const patch = useApp((s) => s.patch);
  const [word, setWord] = useState(selection);
  const [result, setResult] = useState<DictionaryResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const generation = useRef(0);
  const audio = useRef<HTMLAudioElement | null>(null);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (selection) {
      setWord(selection);
      useApp.setState({ selection: '' });
    }
  }, [selection]);
  useEffect(() => {
    input.current?.focus();
    return () => {
      audio.current?.pause();
    };
  }, []);
  const lookup = (query: string, refresh = false) => {
    const request = ++generation.current;
    setLoading(true);
    setError(null);
    void api
      .dictionary(query, refresh)
      .then((value) => {
        if (request === generation.current) setResult(value);
      })
      .catch((e) => {
        if (request === generation.current) {
          setError(String(e));
          setResult(null);
        }
      })
      .finally(() => {
        if (request === generation.current) setLoading(false);
      });
  };
  useEffect(() => {
    if (word.trim().length < 2) {
      generation.current++;
      setResult(null);
      setError(null);
      setLoading(false);
      return;
    }
    const timeout = setTimeout(() => lookup(word), 350);
    return () => {
      clearTimeout(timeout);
      generation.current++;
    };
  }, [word]);
  const first = result?.entries[0];
  const phonetic = first?.phonetic ?? first?.phonetics?.find((p) => p.text)?.text;
  const audioUrl = result?.entries
    .flatMap((e) => e.phonetics ?? [])
    .find((p) => p.audio?.startsWith('https://'))?.audio;
  return (
    <section className="feature dictionary-view">
      <form
        className="search-box"
        onSubmit={(e) => {
          e.preventDefault();
          if (word.trim()) lookup(word);
        }}
      >
        <Icon name="dictionary" />
        <input
          ref={input}
          aria-label="Look up a word"
          placeholder="A word you’re curious about…"
          value={word}
          onChange={(e) => setWord(e.target.value)}
          autoComplete="off"
          spellCheck={false}
        />
        {loading ? (
          <span className="spinner" aria-label="Looking up word" />
        ) : (
          <IconButton
            icon="refresh"
            label="Refresh definition"
            disabled={!word.trim()}
            onClick={() => lookup(word, true)}
          />
        )}
      </form>
      {error ? (
        <ErrorState
          icon="dictionary"
          title="Let’s try another word"
          error={error}
          onRetry={() => lookup(word)}
        />
      ) : result ? (
        <div className="definition-body">
          <div className="word-heading">
            <div>
              <h2>{first?.word ?? result.word}</h2>
              <span className="phonetic">{phonetic}</span>
            </div>
            <span className="spacer" />
            {audioUrl && (
              <IconButton
                icon="volume"
                label="Play pronunciation"
                onClick={() => {
                  audio.current?.pause();
                  audio.current = new Audio(audioUrl);
                  void audio.current
                    .play()
                    .catch(() =>
                      useApp
                        .getState()
                        .notify('Pronunciation audio could not play. Try again online.', true),
                    );
                }}
              />
            )}
            <IconButton
              icon="save"
              label={result.saved ? 'Remove saved word' : 'Save word'}
              active={result.saved}
              onClick={() =>
                void task(() => api.saveWord(result.word, !result.saved)).then((vocabulary) => {
                  if (vocabulary) {
                    patch({ vocabulary });
                    setResult((current) =>
                      current?.word === result.word
                        ? { ...current, saved: !result.saved }
                        : current,
                    );
                  }
                })
              }
            />
          </div>
          <small className="cache-label">
            {result.cached ? 'From your offline cache' : 'Dictionary API · cached for next time'}
          </small>
          {result.entries
            .flatMap((e) => e.meanings)
            .map((meaning, index) => (
              <div className="meaning" key={`${meaning.partOfSpeech}-${index}`}>
                <span className="part-of-speech">{meaning.partOfSpeech}</span>
                {meaning.definitions.map((d, i) => (
                  <div key={i} className="definition">
                    <p>{d.definition}</p>
                    {d.example && <blockquote>{d.example}</blockquote>}
                  </div>
                ))}
                {meaning.synonyms?.length ? (
                  <div className="synonyms">
                    {meaning.synonyms.slice(0, 10).map((s) => (
                      <button key={s} onClick={() => setWord(s)}>
                        {s}
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>
            ))}
        </div>
      ) : !loading ? (
        <>
          <Empty icon="dictionary" title="Find the right word">
            Definitions, pronunciation and examples. Words you look up stay available offline.
          </Empty>
          {data.vocabulary.length > 0 && (
            <div className="saved-words">
              <small>Saved words</small>
              <div className="synonyms">
                {data.vocabulary.map((w) => (
                  <button key={w} onClick={() => setWord(w)}>
                    {w}
                  </button>
                ))}
              </div>
            </div>
          )}
        </>
      ) : (
        <Loading inline label={`Looking up “${word}”…`} />
      )}
    </section>
  );
}
