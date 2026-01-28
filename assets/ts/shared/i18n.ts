/**
 * Internationalization module - loads and provides translations from locales.json.
 * Uses @fluent/bundle for full Fluent syntax support.
 */

import { FluentBundle, FluentResource } from '@fluent/bundle';
import type { FluentVariable } from '@fluent/bundle';

let bundle: FluentBundle | null = null;
let loadPromise: Promise<void> | null = null;

async function load(): Promise<void> {
    if (bundle) return;
    try {
        // Cache-busting: ?t=TIMESTAMP
        const res = await fetch(`/assets/json/locales.json?t=${Date.now()}`);
        
        const data = (await res.json()) as { language: string; resources: string[] };

        bundle = new FluentBundle(data.language);

        for (const resContent of data.resources) {
            const resource = new FluentResource(resContent);
            bundle.addResource(resource);
        }
    } catch (e) {
        console.warn('Failed to load translations:', e);
        bundle = new FluentBundle('en-US');
    }
}

export const translation = {
    /**
     * Initialize translations. Call once at app startup.
     */
    async init(): Promise<void> {
        if (!loadPromise) {
            loadPromise = load();
        }
        await loadPromise;
    },

    /**
     * Get a translation by key.
     * @param key - The translation key (e.g., 'pages', 'share.recap-label')
     * @param args - Optional arguments. If number, treated as { count: n }. If object, passed as is.
     */
    get(key: string, args?: number | Record<string, FluentVariable>): string {
        if (!bundle) return key;

        // Normalize args
        let fluentArgs: Record<string, FluentVariable> | undefined;
        if (typeof args === 'number') {
            fluentArgs = { count: args };
        } else {
            fluentArgs = args;
        }

        // Handle attributes (key.attr)
        let msgId = key;
        let attrId: string | undefined;

        const dotIndex = key.indexOf('.');
        if (dotIndex !== -1) {
            msgId = key.substring(0, dotIndex);
            attrId = key.substring(dotIndex + 1);
        }

        const message = bundle.getMessage(msgId);
        if (!message) return key;

        let pattern;
        if (attrId) {
            pattern = message.attributes?.[attrId];
        } else {
            pattern = message.value;
        }

        if (!pattern) return key;

        return bundle.formatPattern(pattern, fluentArgs);
    },

    /**
     * Get the locale in BCP 47 format.
     */
    getLanguage(): string {
        return bundle?.locales[0] || 'en-US';
    },
};