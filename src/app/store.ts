import { create } from 'zustand';
import { api } from '../lib/api';
import type { Snapshot } from '../lib/types';

import { initialIsland,reduceIsland,type IslandState,type Action } from './islandReducer';
interface State {island:IslandState;data:Snapshot|null;loading:boolean;bootError:string|null;toast:{message:string;error:boolean}|null;selection:string;capturing:boolean;dispatch:(a:Action)=>void;patch:(data:Partial<Snapshot>)=>void;notify:(message:string,error?:boolean)=>void;load:()=>Promise<void>;refreshShelf:()=>Promise<void>}
export const useApp=create<State>((set)=>({
  island:initialIsland,data:null,loading:true,bootError:null,toast:null,selection:'',capturing:false,
  dispatch:(action)=>set(s=>({island:reduceIsland(s.island,action)})),
  patch:(data)=>set(s=>({data:s.data?{...s.data,...data}:null})),
  notify:(message,error=false)=>set({toast:{message,error}}),
  load:async()=>{set({loading:true,bootError:null});try{set({data:await api.bootstrap(),loading:false});}catch(error){set({loading:false,bootError:String(error)});}},
  refreshShelf:async()=>{try{const shelf=await api.shelfList();set(s=>({data:s.data?{...s.data,shelf}:null}));}catch(e){set({toast:{message:String(e),error:true}});}},
}));
export function task(action:()=>Promise<void>,success?:string):Promise<true|undefined>;
export function task<T>(action:()=>Promise<T>,success?:string):Promise<T|undefined>;
export async function task<T>(action:()=>Promise<T>,success?:string):Promise<T|true|undefined>{try{const result=await action();if(success)useApp.getState().notify(success);return result==null?true:result;}catch(error){useApp.getState().notify(String(error),true);return undefined;}}
