import type { IconName } from '../ui/Icon';
import type { Tab } from '../lib/types';
import { ShelfView } from '../modules/shelf/ShelfView';
import { DictionaryView } from '../modules/dictionary/DictionaryView';
import { ScreenTimeView } from '../modules/usage/ScreenTimeView';
import { FocusView } from '../modules/focus/FocusView';
import { NowPlayingView } from '../modules/media/NowPlayingView';
import { NotesView } from '../modules/notes/NotesView';
import { SettingsView } from '../modules/settings/SettingsView';

export const tabs: { id: Tab; label: string; icon: IconName }[] = [
  { id: 'shelf', label: 'Shelf', icon: 'shelf' },
  { id: 'usage', label: 'Screen time', icon: 'usage' },
  { id: 'dictionary', label: 'Dictionary', icon: 'dictionary' },
  { id: 'focus', label: 'Focus timer', icon: 'focus' },
  { id: 'media', label: 'Now Playing', icon: 'media' },
  { id: 'notes', label: 'Scratchpad', icon: 'notes' },
];
export const views = {
  shelf: ShelfView,
  usage: ScreenTimeView,
  dictionary: DictionaryView,
  focus: FocusView,
  media: NowPlayingView,
  notes: NotesView,
  settings: SettingsView,
};
