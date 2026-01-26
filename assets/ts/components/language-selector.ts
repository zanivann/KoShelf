interface LanguageInfo {
    code: string;
    name: string;
}

export async function initLanguageSelector(): Promise<void> {
    // 1. Lógica do Menu Desktop (Abrir/Fechar)
    const settingsBtn = document.getElementById('desktop-settings-btn');
    const settingsMenu = document.getElementById('desktop-settings-menu');

    if (settingsBtn && settingsMenu) {
        // Toggle ao clicar no botão
        settingsBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            settingsMenu.classList.toggle('hidden');
        });

        // Fechar ao clicar fora
        document.addEventListener('click', (e) => {
            const target = e.target as Node;
            if (!settingsMenu.contains(target) && !settingsBtn.contains(target)) {
                settingsMenu.classList.add('hidden');
            }
        });
    }

    // 2. Lógica dos Seletores de Idioma (Desktop & Mobile)
    const desktopSelect = document.getElementById('lang-select') as HTMLSelectElement | null;
    const mobileSelect = document.getElementById('mobile-lang-select') as HTMLSelectElement | null;

    // Filtra para pegar apenas os que existem na página atual
    const selectors = [desktopSelect, mobileSelect].filter((el): el is HTMLSelectElement => el !== null);

    if (selectors.length === 0) return;

    // Helper para cookies
    const getCookie = (name: string): string | null => {
        const value = `; ${document.cookie}`;
        const parts = value.split(`; ${name}=`);
        if (parts.length === 2) return parts.pop()?.split(';').shift() || null;
        return null;
    };

    try {
        const response = await fetch('/api/languages');
        if (!response.ok) throw new Error('Failed to load languages');
        
        const languages: LanguageInfo[] = await response.json();

        // Determina idioma atual uma vez
        const cookieLang = getCookie('koshelf_lang');
        const htmlLang = document.documentElement.lang;
        const currentLang = cookieLang || htmlLang || 'en';

        // Preenche TODOS os seletores encontrados
        selectors.forEach(select => {
            select.innerHTML = '';
            
            languages.forEach((langInfo) => {
                const option = document.createElement('option');
                option.value = langInfo.code;
                option.textContent = langInfo.name; 
                
                if (langInfo.code === currentLang || (currentLang.startsWith(langInfo.code) && langInfo.code.length > 1)) {
                    option.selected = true;
                }
                select.appendChild(option);
            });

            // Adiciona evento de mudança
            select.addEventListener('change', async (e) => {
                const target = e.target as HTMLSelectElement;
                const newLang = target.value;

                // Bloqueia UI
                selectors.forEach(s => s.disabled = true);
                document.body.style.cursor = 'wait';

                try {
                    // 1. Salva Cookie
                    document.cookie = `koshelf_lang=${newLang};path=/;max-age=31536000;SameSite=Strict`;

                    // 2. Chama API
                    const res = await fetch('/api/settings/language', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ lang: newLang })
                    });

                    if (res.ok) {
                        // Limpeza de cache e reload
                        if ('caches' in window) {
                            try {
                                const keyList = await caches.keys();
                                await Promise.all(keyList.map(key => caches.delete(key)));
                            } catch (err) { console.warn(err); }
                        }

                        if ('serviceWorker' in navigator) {
                            const registrations = await navigator.serviceWorker.getRegistrations();
                            for(let registration of registrations) {
                                await registration.unregister();
                            }
                        }

                        const newUrl = new URL(window.location.href);
                        newUrl.searchParams.set('t', Date.now().toString());
                        window.location.href = newUrl.toString();
                    } else {
                        console.error('Server error:', await res.text());
                        selectors.forEach(s => s.disabled = false);
                        document.body.style.cursor = 'default';
                    }
                } catch (err) {
                    console.error(err);
                    selectors.forEach(s => s.disabled = false);
                    document.body.style.cursor = 'default';
                }
            });
        });

    } catch (error) {
        console.error('Language selector error:', error);
        selectors.forEach(s => s.innerHTML = '<option disabled>Error</option>');
    }
}