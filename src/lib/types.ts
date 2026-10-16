export interface FeatureSettings {
  clipboard_history:boolean;copy_into_cache:boolean;shelf_expiry_hours:number;
  watch_downloads:boolean;downloads_folder:string;auto_add_downloads:boolean;
  tracking_paused:boolean;idle_timeout_seconds:number;excluded_apps:string[];track_window_titles:boolean;
  app_limits:Record<string,number>;chrome_executable:string;chrome_profile:string;
  gmail_url:string;outlook_url:string;whatsapp_url:string;
  focus_minutes:number;short_break_minutes:number;long_break_minutes:number;
  presentation_mode:boolean;theme:'dark'|'light'|'system';reduced_motion:boolean;hover_expand:boolean;
  launch_at_startup:boolean;hotkeys:Record<string,string>;
}
export interface ShelfItem {id:string;kind:'file'|'image'|'text'|'link';name:string;path:string|null;content:string|null;size:number;thumbnail:string|null;created_at:number;pinned:boolean;expires_at:number|null;cache_owned:boolean;stack_id:string|null;history:boolean;missing:boolean}
export interface Stack {id:string;name:string}
export interface Note {id:string;body:string;pinned:boolean;updated_at:number}
export interface Timer {phase:'focus'|'short-break'|'long-break';status:'idle'|'running'|'paused'|'completed';duration_seconds:number;remaining_seconds:number;deadline:number|null;completed_focus:number}
export interface MediaState {available:boolean;title:string;artist:string;source:string;playing:boolean;position_seconds:number;duration_seconds:number;artwork:string|null;error:string|null}
export interface Download {id:string;name:string;path:string;size:number}
export interface ChromeSetup {executable:string;profiles:{directory:string;name:string;account:string}[]}
export interface Snapshot {settings:FeatureSettings;shelf:ShelfItem[];stacks:Stack[];notes:Note[];timer:Timer;media:MediaState;vocabulary:string[];chrome:ChromeSetup;downloads:Download[];hotkey_errors:string[];open_on_start?:boolean}
export interface AppUsage {id:string;name:string;path:string;color:string;icon:string|null;seconds:number}
export interface UsageSummary {total_seconds:number;yesterday_seconds:number;apps:AppUsage[];buckets:{label:string;seconds:number;apps:Record<string,number>}[];paused:boolean;range:string}
export interface Definition {definition:string;example?:string;synonyms?:string[]}
export interface DictionaryResult {word:string;cached:boolean;saved:boolean;entries:{word:string;phonetic?:string;phonetics?:{text?:string;audio?:string}[];meanings:{partOfSpeech:string;definitions:Definition[];synonyms?:string[]}[]}[]}
export type Tab='shelf'|'usage'|'dictionary'|'focus'|'media'|'notes'|'settings';
