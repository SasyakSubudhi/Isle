import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { nativeAvailable } from './ipc';
import type { Snapshot,ShelfItem,Stack,Note,DictionaryResult,UsageSummary,Timer,FeatureSettings,MediaState } from './types';

function call<T>(command:string,args?:Record<string,unknown>):Promise<T>{
  if(!nativeAvailable)return Promise.reject(new Error('Open the Isle desktop app to use Windows clipboard, files and tracking.'));
  return invoke<T>(command,args);
}
export const asset=(path:string|null|undefined)=>path?convertFileSrc(path):undefined;
export const api={
  bootstrap:()=>call<Snapshot>('bootstrap'),
  shelfList:()=>call<ShelfItem[]>('shelf_list'),
  addFiles:(paths:string[])=>call<ShelfItem[]>('shelf_add_files',{paths}),
  addText:(text:string)=>call<ShelfItem>('shelf_add_text',{text}),
  paste:()=>call<void>('shelf_paste'),
  remove:(id:string,deleteCache=false)=>call<void>('shelf_remove',{id,deleteCache}),
  update:(id:string,changes:{name?:string;pinned?:boolean;stack?:string;promote?:boolean})=>call<void>('shelf_update',{id,name:changes.name??null,pinned:changes.pinned??null,stack:changes.stack??null,promote:changes.promote??false}),
  copy:(ids:string[])=>call<void>('shelf_copy',{ids}),
  drag:(ids:string[])=>call<void>('shelf_start_drag',{ids}),
  open:(id:string,inFolder=false)=>call<void>('shelf_open',{id,inFolder}),
  clean:(id:string,action:string)=>call<string>('shelf_clean',{id,action}),
  stacks:()=>call<Stack[]>('stacks_list'),
  addStack:(name:string)=>call<Stack[]>('stack_add',{name}),
  removeStack:(id:string)=>call<Stack[]>('stack_remove',{id}),
  notes:()=>call<Note[]>('notes_list'),
  saveNote:(note:Note)=>call<void>('note_save',{note}),
  removeNote:(id:string)=>call<void>('note_remove',{id}),
  dictionary:(word:string,refresh=false)=>call<DictionaryResult>('dictionary_lookup',{word,refresh}),
  saveWord:(word:string,saved:boolean)=>call<string[]>('dictionary_save',{word,saved}),
  usage:(range:string)=>call<UsageSummary>('usage_summary',{range}),
  clearUsage:()=>call<void>('usage_clear'),
  snooze:(id:string)=>call<void>('usage_snooze',{id}),
  timer:(action:string)=>call<Timer>('timer_action',{action}),
  media:(command:string,position?:number)=>call<void>('media_command',{command,position:position??null}),
  chrome:(service:string)=>call<void>('chrome_open',{service}),
  settings:(settings:FeatureSettings)=>call<FeatureSettings>('settings_set',{settings}),
  /** Changes only the given keys, so a stale copy can't undo changes made elsewhere. */
  settingsPatch:(patch:Partial<FeatureSettings>)=>call<FeatureSettings>('settings_patch',{patch}),
  download:(id:string,add:boolean)=>call<void>('download_action',{id,add}),
  capture:()=>call<void>('capture_start'),
  quit:()=>call<void>('app_quit'),
  on:<T>(event:string,handler:(payload:T)=>void)=>listen<T>(event,e=>handler(e.payload)),
  onDrop:(handler:(paths:string[])=>void,onHover:(active:boolean)=>void)=>getCurrentWebviewWindow().onDragDropEvent(e=>{
    if(e.payload.type==='drop'){onHover(false);handler(e.payload.paths);}
    else onHover(e.payload.type==='enter'||e.payload.type==='over');
  }),
};
export const formatTime=(seconds:number)=>{const n=Math.max(0,Math.floor(seconds));return n>=3600?`${Math.floor(n/3600)}h ${Math.floor(n%3600/60)}m`:n>=60?`${Math.floor(n/60)}m`: `${n}s`;};
export const formatClock=(seconds:number)=>`${Math.floor(Math.max(0,seconds)/60).toString().padStart(2,'0')}:${Math.floor(Math.max(0,seconds)%60).toString().padStart(2,'0')}`;
export const fileSize=(bytes:number)=>bytes>=1_048_576?`${(bytes/1_048_576).toFixed(1)} MB`:bytes>=1024?`${Math.round(bytes/1024)} KB`:`${bytes} B`;
export function listenMedia(handler:(media:MediaState)=>void){return api.on('media://changed',handler);}
