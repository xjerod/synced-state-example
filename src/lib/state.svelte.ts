import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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
    #update_latch: boolean = true;
    #un_sub: UnlistenFn | undefined;

    constructor(name: string, object: T) {
        this.name = name;
        this.obj = object;

        listen<T>(`${this.name}_update`, (event) => {
            console.log(`DEBUG [SyncedStore]: ${this.name}_update event`);
            this.#update_latch = true;
            this.obj = event.payload;
        }).then((f) => {
            this.#un_sub = f;
        });

        $effect.root(() => {
            $effect(() => {
                console.log("DEBUG [SyncedStore]: updated...");

                // if (!this.#update_latch) {
                //     invoke(`set_${this.name}`, { new_value: this.object });
                // } else {
                //     console.log("update latch");
                //     this.#update_latch = false;
                // }
            });
        });
    }

    close() {
        if (this.#un_sub) {
            this.#un_sub();
        }
    }

     sync(): boolean {
        console.log(`DEBUG [SyncedStore]: ${this.name} - syncing`, $state.snapshot(this.obj));
        return true
    }
}