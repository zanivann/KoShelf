import '../shared/pwa.js';
import '../shared/dropdown.js';
import '../shared/filter-restore.js';
import '../components/webkit-repaint-hack.js';

// Importa e inicializa
import { initLanguageSelector } from '../components/language-selector.js';

document.addEventListener('DOMContentLoaded', () => {
    initLanguageSelector();
});