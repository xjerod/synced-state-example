import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { commands, events } from "./bindings";

export function NewSyncedState<T extends Record<string, any>>(name: string, object: T): { name: string, obj: T, sync: ()=>boolean, close(): boolean} {
    let proxyObj = $state(object);
    let un_sub: UnlistenFn | undefined;
    listen<T>(`${name}_update`, (event) => {
        console.log(`${name}_update event`);
        proxyObj = event.payload;
    }).then((f) => {
        un_sub = f;
    })

    return {
        name: name,
        obj: proxyObj,
        sync: (): boolean => {
            console.log(`${name} - syncing`, $state.snapshot(proxyObj));
            return true
        },
        close: (): boolean => {
            console.log(`${name} - closing`);
            if (un_sub) {
                un_sub();
            }
            return true
        }
    };
}

export class SyncedState<T> {
    name: string;
    obj: T = $state({} as T);
    #un_sub: UnlistenFn | undefined;

    constructor(name: string, object: T) {
        this.name = name;
        this.obj = object;

        listen<T>(`${this.name}_update`, (event) => {
            console.log(`DEBUG [SyncedStore]: ${this.name}_update event`,event.payload);
            this.obj = event.payload;
        }).then((f) => {
            this.#un_sub = f;
            commands.getState(this.name);
        });
    }

    close() {
        if (this.#un_sub) {
            this.#un_sub();
        }
    }

    async sync():  Promise<boolean> {
        const val = $state.snapshot(this.obj);
        console.log(`DEBUG [SyncedStore]: ${this.name} - syncing`, val);
        await events.update.emit({version: null, name:this.name, value: JSON.stringify(val)});
        return true
    }
}