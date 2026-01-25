// Define a interface para o objeto que vem do Rust
interface LanguageInfo {
    code: string;
    name: string;
}

export async function initLanguageSelector(): Promise<void> {
    const langSelect = document.getElementById('lang-select') as HTMLSelectElement;

    if (!langSelect) return;

    // Helper to get cookie
    const getCookie = (name: string): string | null => {
        const value = `; ${document.cookie}`;
        const parts = value.split(`; ${name}=`);
        if (parts.length === 2) return parts.pop()?.split(';').shift() || null;
        return null;
    };

    try {
        const response = await fetch('/api/languages');
        if (!response.ok) throw new Error('Failed to load languages');
        
        // AGORA RECEBEMOS OBJETOS, NÃO STRINGS
        const languages: LanguageInfo[] = await response.json();

        langSelect.innerHTML = '';

        const cookieLang = getCookie('koshelf_lang');
        const htmlLang = document.documentElement.lang;
        const currentLang = cookieLang || htmlLang || 'en';

        languages.forEach((langInfo) => {
            const option = document.createElement('option');
            
            // O valor interno continua sendo o código (pt, en)
            option.value = langInfo.code;
            
            // O texto visível agora é o nome bonito vindo do FTL
            option.textContent = langInfo.name; 
            
            // Lógica de seleção
            if (langInfo.code === currentLang || (currentLang.startsWith(langInfo.code) && langInfo.code.length > 1)) {
                option.selected = true;
            }
            langSelect.appendChild(option);
        });

        langSelect.addEventListener('change', async (e) => {
            const target = e.target as HTMLSelectElement;
            const newLang = target.value;

            target.disabled = true;
            document.body.style.cursor = 'wait';

            try {
                // 1. Save Cookie
                document.cookie = `koshelf_lang=${newLang};path=/;max-age=31536000;SameSite=Strict`;

                // 2. Call API
                const res = await fetch('/api/settings/language', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ lang: newLang })
                });

                if (res.ok) {
                    // 3. Clear PWA Cache
                    if ('caches' in window) {
                        try {
                            const keyList = await caches.keys();
                            await Promise.all(keyList.map(key => caches.delete(key)));
                            console.log('PWA Cache cleared');
                        } catch (err) {
                            console.warn('Failed to clear caches', err);
                        }
                    }

                    // 4. Unregister Workers
                    if ('serviceWorker' in navigator) {
                        const registrations = await navigator.serviceWorker.getRegistrations();
                        for(let registration of registrations) {
                            await registration.unregister();
                        }
                    }

                    // 5. Hard Reload
                    const newUrl = new URL(window.location.href);
                    newUrl.searchParams.set('t', Date.now().toString());
                    window.location.href = newUrl.toString();

                } else {
                    console.error('Server error:', await res.text());
                    alert("Error updating language.");
                    target.disabled = false;
                    document.body.style.cursor = 'default';
                }
            } catch (err) {
                console.error('Connection error:', err);
                target.disabled = false;
                document.body.style.cursor = 'default';
            }
        });

    } catch (error) {
        console.error('Language selector error:', error);
        langSelect.innerHTML = '<option disabled>Error</option>';
    }
}