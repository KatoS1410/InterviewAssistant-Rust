"""
app.py — KatoS Interview Assistant UI.

Вкладки:
  1. Главная  — запись / транскрипция / ответ ИИ
  2. Настройки — все параметры

Горячие клавиши (глобальные):
  ← держать  — запись системного звука (Desktop)
  → держать  — запись микрофона (Mic)
"""

from __future__ import annotations

import logging
import threading
import tkinter as tk
import tkinter.filedialog as fd
import tkinter.messagebox as mb
from tkinter import ttk
from typing import Optional

from . import config as cfg_module
from .ai_client import session as ai_session
from .audio import AudioRecorder, find_loopback_device, list_all_input_devices, SAMPLE_RATE
from .transcriber import transcriber

log = logging.getLogger(__name__)

# ─────────────────────────── Цветовая схема ───────────────────────────────

C = {
    "bg":         "#1a1b2e",
    "bg2":        "#252640",
    "bg3":        "#2e3050",
    "bg4":        "#383a5c",
    "accent":     "#6c63ff",
    "accent_h":   "#4e46d4",
    "danger":     "#ff6b8a",
    "success":    "#4ecca3",
    "warning":    "#ffd166",
    "text":       "#e8e8f0",
    "text_dim":   "#7b7d9e",
    "border":     "#3d3f6b",
    "rec":        "#ff4757",
}

FONT      = ("Segoe UI", 10)
FONT_B    = ("Segoe UI", 10, "bold")
FONT_SM   = ("Segoe UI", 9)
FONT_LG   = ("Segoe UI", 13, "bold")
FONT_MONO = ("Consolas", 10)


# ─────────────────────────── TTK стили ────────────────────────────────────

def _style():
    s = ttk.Style()
    s.theme_use("clam")

    s.configure(".", background=C["bg"], foreground=C["text"],
                fieldbackground=C["bg3"], bordercolor=C["border"],
                font=FONT)
    s.configure("TFrame", background=C["bg"])
    s.configure("TLabel", background=C["bg"], foreground=C["text"])
    s.configure("Dim.TLabel", background=C["bg"], foreground=C["text_dim"], font=FONT_SM)

    # Вкладки — крупные, без эмодзи путаницы
    s.configure("TNotebook", background=C["bg2"], borderwidth=0, tabmargins=[0, 4, 0, 0])
    s.configure("TNotebook.Tab",
        background=C["bg3"], foreground=C["text_dim"],
        padding=[22, 8], font=("Segoe UI", 10, "bold"),
    )
    s.map("TNotebook.Tab",
        background=[("selected", C["accent"])],
        foreground=[("selected", "#ffffff")],
    )

    # Основная кнопка
    s.configure("TButton", background=C["accent"], foreground="#ffffff",
                borderwidth=0, relief="flat", padding=[14, 7], font=FONT_B)
    s.map("TButton",
        background=[("active", C["accent_h"]), ("pressed", C["accent_h"]),
                    ("disabled", C["bg3"])],
        foreground=[("disabled", C["text_dim"])],
        relief=[("pressed", "flat")],
    )
    # Опасная кнопка
    s.configure("Danger.TButton", background=C["danger"], foreground="#1a1b2e")
    s.map("Danger.TButton", background=[("active", "#d94060"), ("pressed", "#b03050")])

    # Тихая кнопка (второстепенная)
    s.configure("Ghost.TButton", background=C["bg3"], foreground=C["text"],
                borderwidth=0, relief="flat", padding=[10, 5], font=FONT)
    s.map("Ghost.TButton",
        background=[("active", C["bg4"]), ("pressed", C["border"])],
    )

    s.configure("TEntry", fieldbackground=C["bg3"], foreground=C["text"],
                insertcolor=C["text"], bordercolor=C["border"],
                relief="flat", padding=[6, 4])
    s.configure("TRadiobutton", background=C["bg"], foreground=C["text"],
                indicatorcolor=C["accent"])
    s.map("TRadiobutton", background=[("active", C["bg"])])
    s.configure("TCheckbutton", background=C["bg"], foreground=C["text"])
    s.configure("TCombobox", fieldbackground=C["bg3"], foreground=C["text"],
                selectbackground=C["accent"], selectforeground="#fff",
                background=C["bg3"], arrowcolor=C["text"])
    s.map("TCombobox", fieldbackground=[("readonly", C["bg3"])])
    s.configure("TScrollbar", background=C["bg2"], troughcolor=C["bg"],
                bordercolor=C["bg"], arrowcolor=C["text_dim"], relief="flat")
    s.configure("TLabelframe", background=C["bg"], bordercolor=C["border"])
    s.configure("TLabelframe.Label", background=C["bg"],
                foreground=C["accent"], font=FONT_B)
    s.configure("TSeparator", background=C["border"])


# ─────────────────────────── Виджеты-помощники ────────────────────────────

def _scrolled_text(parent, height=8, readonly=False, **kw) -> tuple[tk.Frame, tk.Text]:
    frame = tk.Frame(parent, bg=C["border"], bd=1)
    state = "disabled" if readonly else "normal"
    t = tk.Text(
        frame, height=height,
        bg=C["bg3"], fg=C["text"],
        insertbackground=C["text"],
        selectbackground=C["accent"], selectforeground="#fff",
        relief="flat", borderwidth=0,
        font=FONT_MONO, wrap="word",
        padx=10, pady=8, state=state, **kw,
    )
    sb = ttk.Scrollbar(frame, command=t.yview)
    t.configure(yscrollcommand=sb.set)
    t.pack(side="left", fill="both", expand=True)
    sb.pack(side="right", fill="y")
    # Скролл мышкой
    t.bind("<MouseWheel>", lambda e: t.yview_scroll(int(-e.delta / 60), "units"))
    return frame, t


def _add_scroll(canvas: tk.Canvas) -> None:
    """Включить скролл мышью для canvas (используется в настройках)."""
    def _scroll(e):
        canvas.yview_scroll(int(-e.delta / 60), "units")
    canvas.bind_all("<MouseWheel>", _scroll)


def _lbl(parent, text, bold=False, dim=False, size=10, **kw) -> ttk.Label:
    font = ("Segoe UI", size, "bold") if bold else ("Segoe UI", size)
    style = "Dim.TLabel" if dim else "TLabel"
    return ttk.Label(parent, text=text, style=style, font=font, **kw)


def _btn(parent, text, cmd, style="TButton", **kw) -> ttk.Button:
    b = ttk.Button(parent, text=text, command=cmd, style=style, **kw)
    # Анимация нажатия через цвет
    b.bind("<ButtonPress-1>",   lambda e: b.state(["pressed"]))
    b.bind("<ButtonRelease-1>", lambda e: b.state(["!pressed"]))
    return b


def _entry(parent, var=None, password=False, **kw) -> ttk.Entry:
    show = "*" if password else ""
    return ttk.Entry(parent, textvariable=var, show=show, **kw)


# ─────────────────────────── Главное окно ─────────────────────────────────

class App(tk.Tk):
    def __init__(self):
        super().__init__()

        self.title("KatoS Interview Assistant")
        self.geometry("1020x720")
        self.minsize(820, 580)
        self.configure(bg=C["bg"])
        _style()

        self._cfg = cfg_module.load()
        self._model_loaded = False
        self._ai_ready = False
        self._mic_recorder: Optional[AudioRecorder] = None
        self._desk_recorder: Optional[AudioRecorder] = None
        self._loopback_idx: Optional[int] = None
        self._ai_lock = threading.Lock()
        self._all_devices: list[dict] = []  # кэш устройств

        self._build_ui()
        self._apply_config()
        self._setup_hotkeys()

        self.protocol("WM_DELETE_WINDOW", self._on_close)

    # ─────────────── Построение UI ────────────────────────────────────────

    def _build_ui(self):
        # ── Заголовок ──────────────────────────────────────────────────
        header = tk.Frame(self, bg=C["bg2"], pady=10)
        header.pack(fill="x")

        tk.Label(
            header, text="KatoS Interview Assistant",
            bg=C["bg2"], fg=C["accent"], font=("Segoe UI", 14, "bold"),
        ).pack(side="left", padx=18)

        # Индикатор справа
        self._dot = tk.Label(header, text="●", bg=C["bg2"],
                              fg=C["text_dim"], font=("Segoe UI", 16))
        self._dot.pack(side="right", padx=6)
        self._hdr_status = tk.Label(header, text="Не готово", bg=C["bg2"],
                                     fg=C["text_dim"], font=FONT_SM)
        self._hdr_status.pack(side="right", padx=2)

        # ── Вкладки ────────────────────────────────────────────────────
        nb = ttk.Notebook(self)
        nb.pack(fill="both", expand=True)

        tab_main = ttk.Frame(nb)
        tab_cfg  = ttk.Frame(nb, padding=0)

        nb.add(tab_main, text="   Главная   ")
        nb.add(tab_cfg,  text="   Настройки   ")

        self._build_main(tab_main)
        self._build_settings(tab_cfg)

        # ── Статус-бар снизу ───────────────────────────────────────────
        bar = tk.Frame(self, bg=C["bg2"], pady=4)
        bar.pack(fill="x", side="bottom")
        self._statusbar = tk.Label(bar, text="Загрузи модель и настрой ИИ",
                                    bg=C["bg2"], fg=C["text_dim"], font=FONT_SM)
        self._statusbar.pack(side="left", padx=14)

    # ─────────────── ВКЛАДКА: ГЛАВНАЯ ─────────────────────────────────────

    def _build_main(self, parent):
        # Верхняя панель: позиция + кнопка загрузки
        top = tk.Frame(parent, bg=C["bg2"], pady=10, padx=14)
        top.pack(fill="x")

        _lbl(top, "Должность:").pack(side="left", padx=(0, 6))
        self._var_pos = tk.StringVar()
        _entry(top, var=self._var_pos, width=28).pack(side="left", padx=(0, 14))

        self._load_btn = _btn(top, "  Загрузить модель  ", self._on_load_click)
        self._load_btn.pack(side="left", padx=(0, 10))

        self._rec_label = tk.Label(top, text="", bg=C["bg2"],
                                    fg=C["rec"], font=FONT_B)
        self._rec_label.pack(side="left", padx=(6, 0))

        # Пояснение по клавишам
        hint = tk.Label(top,
            text="←  Системный звук    →  Микрофон    (держи клавишу)",
            bg=C["bg2"], fg=C["text_dim"], font=FONT_SM)
        hint.pack(side="right", padx=14)

        # ── Разделённая панель: транскрипция | ответ ───────────────────
        paned = tk.PanedWindow(parent, orient="horizontal",
                               bg=C["border"], sashwidth=5, sashpad=0,
                               sashrelief="flat")
        paned.pack(fill="both", expand=True)

        # Левая — транскрипция
        left = tk.Frame(paned, bg=C["bg"])
        paned.add(left, stretch="always", minsize=320)

        lh = tk.Frame(left, bg=C["bg"], pady=8, padx=12)
        lh.pack(fill="x")
        _lbl(lh, "Транскрипция", bold=True, size=11).pack(side="left")
        self._char_count = tk.Label(lh, text="0 симв.", bg=C["bg"],
                                     fg=C["text_dim"], font=FONT_SM)
        self._char_count.pack(side="right")

        sf, self._tr_text = _scrolled_text(left, height=14)
        sf.pack(fill="both", expand=True, padx=12, pady=(0, 6))
        self._tr_text.bind("<<Modified>>", self._on_transcript_change)

        lr = tk.Frame(left, bg=C["bg"], pady=6, padx=12)
        lr.pack(fill="x")
        _btn(lr, "Отправить вручную", self._on_send_manual).pack(side="left")
        _btn(lr, "Очистить", self._clear_transcript,
             style="Ghost.TButton").pack(side="left", padx=8)

        # Правая — ответ ИИ
        right = tk.Frame(paned, bg=C["bg"])
        paned.add(right, stretch="always", minsize=320)

        rh = tk.Frame(right, bg=C["bg"], pady=8, padx=12)
        rh.pack(fill="x")
        _lbl(rh, "Ответ ИИ", bold=True, size=11).pack(side="left")
        self._thinking = tk.Label(rh, text="", bg=C["bg"],
                                   fg=C["accent"], font=FONT_SM)
        self._thinking.pack(side="right")

        sf2, self._ai_text = _scrolled_text(right, height=14, readonly=True)
        sf2.pack(fill="both", expand=True, padx=12, pady=(0, 6))

        rr = tk.Frame(right, bg=C["bg"], pady=6, padx=12)
        rr.pack(fill="x")
        _btn(rr, "Копировать", self._copy_response).pack(side="left")
        _btn(rr, "Очистить", self._clear_response,
             style="Ghost.TButton").pack(side="left", padx=8)
        _btn(rr, "Очистить историю", self._clear_history,
             style="Ghost.TButton").pack(side="right")

    # ─────────────── ВКЛАДКА: НАСТРОЙКИ ───────────────────────────────────

    def _build_settings(self, parent):
        canvas = tk.Canvas(parent, bg=C["bg"], highlightthickness=0)
        vsb = ttk.Scrollbar(parent, orient="vertical", command=canvas.yview)
        canvas.configure(yscrollcommand=vsb.set)
        vsb.pack(side="right", fill="y")
        canvas.pack(side="left", fill="both", expand=True)

        inner = tk.Frame(canvas, bg=C["bg"])
        win_id = canvas.create_window((0, 0), window=inner, anchor="nw")

        def _on_inner(e):
            canvas.configure(scrollregion=canvas.bbox("all"))
        def _on_canvas(e):
            canvas.itemconfig(win_id, width=e.width)

        inner.bind("<Configure>", _on_inner)
        canvas.bind("<Configure>", _on_canvas)
        # Скролл мышью на всей вкладке
        canvas.bind("<MouseWheel>",
                    lambda e: canvas.yview_scroll(int(-e.delta / 60), "units"))
        inner.bind("<MouseWheel>",
                   lambda e: canvas.yview_scroll(int(-e.delta / 60), "units"))

        pad = {"padx": 20, "fill": "x"}

        # ── VOSK ────────────────────────────────────────────────────────
        lf1 = ttk.LabelFrame(inner, text=" Распознавание речи (VOSK) ", padding=14)
        lf1.pack(**pad, pady=(16, 14))

        _lbl(lf1, "Путь к папке с моделью VOSK:").grid(row=0, column=0, sticky="w", pady=3)
        _lbl(lf1,
             "Скачать модели: https://alphacephei.com/vosk/models   "
             "(рекомендуется vosk-model-ru-0.42)",
             dim=True).grid(row=1, column=0, columnspan=3, sticky="w")

        self._var_model = tk.StringVar()
        _entry(lf1, var=self._var_model, width=52).grid(
            row=2, column=0, sticky="ew", pady=6)
        _btn(lf1, "  Обзор...  ", self._browse_model,
             style="Ghost.TButton").grid(row=2, column=1, padx=8)
        lf1.columnconfigure(0, weight=1)

        # ── ИИ бэкенд ───────────────────────────────────────────────────
        lf2 = ttk.LabelFrame(inner, text=" Искусственный интеллект ", padding=14)
        lf2.pack(**pad, pady=(0, 14))

        self._var_backend = tk.StringVar(value="openai")
        rb_row = tk.Frame(lf2, bg=C["bg"])
        rb_row.pack(fill="x", pady=(0, 10))

        ttk.Radiobutton(rb_row, text="OpenAI / совместимый API",
                        variable=self._var_backend, value="openai",
                        command=self._on_backend_change).pack(side="left", padx=(0, 30))
        ttk.Radiobutton(rb_row, text="Ollama (локально, бесплатно)",
                        variable=self._var_backend, value="ollama",
                        command=self._on_backend_change).pack(side="left")

        # OpenAI поля
        self._frm_openai = tk.Frame(lf2, bg=C["bg"])
        self._frm_openai.pack(fill="x")
        oa_fields = [
            ("API ключ (sk-...):", "_var_oa_key", True),
            ("Модель:", "_var_oa_model", False),
            ("Base URL:", "_var_oa_url", False),
        ]
        for i, (lbl, attr, pwd) in enumerate(oa_fields):
            _lbl(self._frm_openai, lbl).grid(row=i, column=0, sticky="w", pady=4, padx=(0, 10))
            setattr(self, attr, tk.StringVar())
            _entry(self._frm_openai, var=getattr(self, attr),
                   width=50, password=pwd).grid(row=i, column=1, sticky="ew", pady=4)
        self._frm_openai.columnconfigure(1, weight=1)

        # Ollama поля
        self._frm_ollama = tk.Frame(lf2, bg=C["bg"])
        ol_fields = [
            ("Ollama URL:", "_var_ol_url"),
            ("Модель:", "_var_ol_model"),
        ]
        for i, (lbl, attr) in enumerate(ol_fields):
            _lbl(self._frm_ollama, lbl).grid(row=i, column=0, sticky="w", pady=4, padx=(0, 10))
            setattr(self, attr, tk.StringVar())
            _entry(self._frm_ollama, var=getattr(self, attr),
                   width=50).grid(row=i, column=1, sticky="ew", pady=4)
        self._frm_ollama.columnconfigure(1, weight=1)

        test_row = tk.Frame(lf2, bg=C["bg"])
        test_row.pack(fill="x", pady=(12, 0))
        _btn(test_row, "  Проверить соединение  ", self._test_connection).pack(side="left")
        self._conn_label = tk.Label(test_row, text="", bg=C["bg"],
                                     fg=C["text_dim"], font=FONT_SM, wraplength=500, justify="left")
        self._conn_label.pack(side="left", padx=12)

        # ── Аудио устройства ────────────────────────────────────────────
        lf3 = ttk.LabelFrame(inner, text=" Аудио устройства ", padding=14)
        lf3.pack(**pad, pady=(0, 14))

        # Длина записи
        row0 = tk.Frame(lf3, bg=C["bg"])
        row0.pack(fill="x", pady=(0, 10))
        _lbl(row0, "Макс. длина записи (секунды):").pack(side="left")
        self._var_maxrec = tk.StringVar(value="20")
        _entry(row0, var=self._var_maxrec, width=6).pack(side="left", padx=10)

        # Loopback
        _lbl(lf3, "Устройство для записи системного звука (Desktop ←):").pack(anchor="w")
        _lbl(lf3,
             "Если Stereo Mix / Стерео Микшер не отображается — включи его в настройках звука Windows\n"
             "(ПКМ на иконке звука → Параметры звука → Дополнительные параметры → Запись).",
             dim=True).pack(anchor="w", pady=(2, 8))

        dev_row = tk.Frame(lf3, bg=C["bg"])
        dev_row.pack(fill="x")

        self._var_loopback = tk.StringVar()
        self._cb_devices = ttk.Combobox(dev_row, textvariable=self._var_loopback,
                                         state="readonly", width=50)
        self._cb_devices.pack(side="left")
        _btn(dev_row, "  Обновить список  ", self._refresh_devices,
             style="Ghost.TButton").pack(side="left", padx=8)

        self._loopback_status = tk.Label(lf3, text="", bg=C["bg"],
                                          fg=C["text_dim"], font=FONT_SM)
        self._loopback_status.pack(anchor="w", pady=(6, 0))

        # Микрофон
        _lbl(lf3, "\nУстройство микрофона (Mic →):").pack(anchor="w")
        mic_row = tk.Frame(lf3, bg=C["bg"])
        mic_row.pack(fill="x", pady=(4, 0))
        self._var_mic = tk.StringVar()
        self._cb_mic = ttk.Combobox(mic_row, textvariable=self._var_mic,
                                     state="readonly", width=50)
        self._cb_mic.pack(side="left")
        _lbl(mic_row, "  (оставь пустым = устройство по умолчанию)",
             dim=True).pack(side="left")

        # ── Системный промпт ────────────────────────────────────────────
        lf4 = ttk.LabelFrame(inner, text=" Системный промпт ", padding=14)
        lf4.pack(**pad, pady=(0, 14))

        _lbl(lf4, "Используй {position} — будет заменено на должность из главной вкладки.",
             dim=True).pack(anchor="w")
        sf, self._prompt_text = _scrolled_text(lf4, height=5)
        sf.pack(fill="x", pady=(8, 0))

        # ── Кнопка сохранить ────────────────────────────────────────────
        save_row = tk.Frame(inner, bg=C["bg"])
        save_row.pack(fill="x", padx=20, pady=(0, 20))
        _btn(save_row, "  Сохранить настройки  ", self._save_settings).pack(side="left")
        self._save_lbl = tk.Label(save_row, text="", bg=C["bg"],
                                   fg=C["success"], font=FONT_SM)
        self._save_lbl.pack(side="left", padx=14)

    # ─────────────── Применение конфига ───────────────────────────────────

    def _apply_config(self):
        c = self._cfg
        self._var_pos.set(c.get("position", ""))
        self._var_model.set(c.get("vosk_model_path", ""))
        self._var_backend.set(c.get("ai_backend", "openai"))
        self._var_oa_key.set(c.get("openai_api_key", ""))
        self._var_oa_model.set(c.get("openai_model", "gpt-4o-mini"))
        self._var_oa_url.set(c.get("openai_base_url", "https://api.openai.com/v1"))
        self._var_ol_url.set(c.get("ollama_base_url", "http://localhost:11434"))
        self._var_ol_model.set(c.get("ollama_model", "llama3"))
        self._var_maxrec.set(str(c.get("max_record_seconds", 20)))

        prompt = c.get("system_prompt_template", "")
        self._prompt_text.insert("1.0", prompt)

        self._on_backend_change()
        self._refresh_devices()

    def _on_backend_change(self):
        if self._var_backend.get() == "openai":
            self._frm_ollama.pack_forget()
            self._frm_openai.pack(fill="x")
        else:
            self._frm_openai.pack_forget()
            self._frm_ollama.pack(fill="x")

    # ─────────────── Устройства ───────────────────────────────────────────

    def _refresh_devices(self):
        """Обновить список устройств и автовыбрать loopback."""
        try:
            self._all_devices = list_all_input_devices()
        except Exception as e:
            self._loopback_status.config(
                text=f"Ошибка получения устройств: {e}", fg=C["danger"])
            return

        if not self._all_devices:
            self._loopback_status.config(text="Нет входных устройств", fg=C["danger"])
            return

        names = [f"[{d['index']}] {d['name']}" for d in self._all_devices]

        self._cb_devices["values"] = names
        self._cb_mic["values"] = ["[авто] Устройство по умолчанию"] + names

        # Восстановить сохранённый loopback индекс
        saved_idx = self._cfg.get("loopback_device_index", -1)
        auto_idx = find_loopback_device()

        if saved_idx >= 0:
            match = next(
                (n for n in names if n.startswith(f"[{saved_idx}]")), None)
            if match:
                self._cb_devices.set(match)
                self._loopback_idx = saved_idx
                self._loopback_status.config(
                    text=f"Выбрано из настроек: {match}", fg=C["success"])
                return

        if auto_idx is not None:
            match = next(
                (n for n in names if n.startswith(f"[{auto_idx}]")), None)
            if match:
                self._cb_devices.set(match)
                self._loopback_idx = auto_idx
                self._loopback_status.config(
                    text=f"Найдено автоматически: {match}", fg=C["success"])
        else:
            self._cb_devices.set("")
            self._loopback_idx = None
            self._loopback_status.config(
                text="Loopback не найден автоматически — выбери вручную из списка выше.",
                fg=C["warning"])

    def _get_selected_loopback(self) -> Optional[int]:
        """Получить выбранный loopback индекс из комбобокса."""
        val = self._var_loopback.get().strip()
        if not val:
            return None
        try:
            idx = int(val.split("]")[0].lstrip("["))
            self._loopback_idx = idx
            return idx
        except (ValueError, IndexError):
            return None

    def _get_selected_mic(self) -> Optional[int]:
        """Получить выбранный микрофон или None (дефолт)."""
        val = self._var_mic.get().strip()
        if not val or val.startswith("[авто]"):
            return None
        try:
            return int(val.split("]")[0].lstrip("["))
        except (ValueError, IndexError):
            return None

    # ─────────────── Горячие клавиши ──────────────────────────────────────

    def _setup_hotkeys(self):
        try:
            from pynput import keyboard as kb
            _held = set()

            def on_press(key):
                if key in _held:
                    return
                _held.add(key)
                if key == kb.Key.left:
                    self.after(0, self._start_desk)
                elif key == kb.Key.right:
                    self.after(0, self._start_mic)

            def on_release(key):
                _held.discard(key)
                if key == kb.Key.left:
                    self.after(0, self._stop_desk)
                elif key == kb.Key.right:
                    self.after(0, self._stop_mic)

            listener = kb.Listener(on_press=on_press, on_release=on_release)
            listener.daemon = True
            listener.start()
            self._kb_listener = listener
        except Exception as e:
            log.error("Хоткеи: %s", e)
            self._set_status(f"Хоткеи недоступны: {e}", "warning")

    # ─────────────── Запись Desktop ───────────────────────────────────────

    def _start_desk(self):
        if not self._model_loaded:
            return
        if (self._desk_recorder and self._desk_recorder.is_recording) or \
           (self._mic_recorder and self._mic_recorder.is_recording):
            return

        dev = self._get_selected_loopback()
        if dev is None:
            self._set_status(
                "Loopback устройство не выбрано! Открой Настройки → Аудио устройства.", "danger")
            return

        try:
            max_s = int(self._var_maxrec.get())
        except ValueError:
            max_s = 20

        self._desk_recorder = AudioRecorder(device_index=dev)
        try:
            self._desk_recorder.start(max_seconds=max_s)
            self._rec_label.config(text="● REC  Desktop")
            self._set_status("Запись системного звука...", "warning")
        except Exception as e:
            err = str(e)
            self._desk_recorder = None
            self._set_status(f"Ошибка записи Desktop: {err}", "danger")

    def _stop_desk(self):
        if self._desk_recorder and self._desk_recorder.is_recording:
            self._rec_label.config(text="")
            audio = self._desk_recorder.stop()
            self._desk_recorder = None
            if len(audio) > 0:
                threading.Thread(target=self._process_audio,
                                 args=(audio,), daemon=True).start()
            else:
                self._set_status("Запись пустая", "warning")

    # ─────────────── Запись Mic ───────────────────────────────────────────

    def _start_mic(self):
        if not self._model_loaded:
            return
        if (self._desk_recorder and self._desk_recorder.is_recording) or \
           (self._mic_recorder and self._mic_recorder.is_recording):
            return

        try:
            max_s = int(self._var_maxrec.get())
        except ValueError:
            max_s = 20

        mic_dev = self._get_selected_mic()
        self._mic_recorder = AudioRecorder(device_index=mic_dev)
        try:
            self._mic_recorder.start(max_seconds=max_s)
            self._rec_label.config(text="● REC  Mic")
            self._set_status("Запись микрофона...", "warning")
        except Exception as e:
            err = str(e)
            self._mic_recorder = None
            self._set_status(f"Ошибка записи Mic: {err}", "danger")

    def _stop_mic(self):
        if self._mic_recorder and self._mic_recorder.is_recording:
            self._rec_label.config(text="")
            audio = self._mic_recorder.stop()
            self._mic_recorder = None
            if len(audio) > 0:
                threading.Thread(target=self._process_audio,
                                 args=(audio,), daemon=True).start()
            else:
                self._set_status("Запись пустая", "warning")

    # ─────────────── Аудио → VOSK → ИИ ──────────────────────────────────

    def _process_audio(self, audio):
        self.after(0, lambda: self._set_status("Распознаю речь...", "warning"))
        try:
            text = transcriber.transcribe(audio)
        except Exception as e:
            err = str(e)
            self.after(0, lambda: self._set_status(f"Ошибка VOSK: {err}", "danger"))
            return

        if not text.strip():
            self.after(0, lambda: self._set_status("Речь не распознана (тишина?)", "warning"))
            return

        def _ui_update():
            self._set_transcript(text)
            self._send_to_ai(text)

        self.after(0, _ui_update)

    def _set_transcript(self, text: str):
        self._tr_text.delete("1.0", "end")
        self._tr_text.insert("1.0", text)
        n = len(text)
        self._char_count.config(text=f"{n} симв.")

    def _on_transcript_change(self, event=None):
        try:
            text = self._tr_text.get("1.0", "end-1c")
            self._char_count.config(text=f"{len(text)} симв.")
            self._tr_text.edit_modified(False)
        except Exception:
            pass

    # ─────────────── Отправка в ИИ ───────────────────────────────────────

    def _send_to_ai(self, text: str):
        if not self._ai_ready:
            self._set_status("ИИ не готов — нажми 'Загрузить модель'", "danger")
            return

        if not self._ai_lock.acquire(blocking=False):
            self._set_status("ИИ уже обрабатывает запрос, подожди...", "warning")
            return

        self._ai_text.config(state="normal")
        self._ai_text.delete("1.0", "end")
        self._ai_text.config(state="disabled")
        self._thinking.config(text="⏳ Думаю...")
        self._set_status("Запрос к ИИ...", "warning")

        def worker():
            try:
                def on_token(tok: str):
                    self.after(0, lambda t=tok: self._append_ai(t))

                ai_session.ask(text, on_token=on_token)
                self.after(0, self._ai_done)
            except Exception as exc:
                # ВАЖНО: захватываем exc в переменную ДО lambda
                # (Python 3.12+ удаляет 'e' из except блока)
                err_msg = str(exc)
                self.after(0, lambda m=err_msg: self._ai_error(m))
            finally:
                self._ai_lock.release()

        threading.Thread(target=worker, daemon=True).start()

    def _append_ai(self, token: str):
        self._ai_text.config(state="normal")
        self._ai_text.insert("end", token)
        self._ai_text.see("end")
        self._ai_text.config(state="disabled")

    def _ai_done(self):
        self._thinking.config(text="")
        self._set_status("Готово ✓", "success")

    def _ai_error(self, msg: str):
        self._thinking.config(text="")
        self._ai_text.config(state="normal")
        self._ai_text.insert("end", f"\n\n[ОШИБКА] {msg}")
        self._ai_text.config(state="disabled")
        self._set_status(f"Ошибка ИИ: {msg[:80]}", "danger")

    # ─────────────── Кнопки ──────────────────────────────────────────────

    def _on_load_click(self):
        model_path = self._var_model.get().strip()
        position   = self._var_pos.get().strip()

        if not model_path:
            mb.showerror("Ошибка", "Укажи путь к модели VOSK в настройках.")
            return
        if not position:
            mb.showerror("Ошибка",
                "Введи должность в поле 'Должность'\n"
                "(например: Python разработчик, Backend Engineer).")
            return

        self._load_btn.config(state="disabled",
                               text="  Загружаю...  ")
        self._set_status("Загрузка модели VOSK...", "warning")

        def worker():
            # 1. VOSK
            try:
                transcriber.load(
                    model_path,
                    on_progress=lambda m: self.after(
                        0, lambda msg=m: self._set_status(msg, "warning")),
                )
                self._model_loaded = True
            except Exception as exc:
                err = str(exc)
                self.after(0, lambda m=err: self._load_fail(f"VOSK: {m}"))
                return

            # 2. Конфигурируем ИИ клиент
            try:
                self._configure_ai()
            except Exception as exc:
                err = str(exc)
                self.after(0, lambda m=err: self._load_fail(f"ИИ конфиг: {m}"))
                return

            # 3. Инит сессии
            try:
                prompt = self._prompt_text.get("1.0", "end").strip()
                ai_session.init_session(position, prompt)
                self._ai_ready = True
            except Exception as exc:
                err = str(exc)
                self.after(0, lambda m=err: self._load_fail(f"Сессия: {m}"))
                return

            self.after(0, self._load_ok)

        threading.Thread(target=worker, daemon=True).start()

    def _configure_ai(self):
        backend = self._var_backend.get()
        if backend == "openai":
            key = self._var_oa_key.get().strip()
            if not key:
                raise ValueError("Не указан OpenAI API ключ")
            ai_session.configure_openai(
                key,
                self._var_oa_model.get().strip(),
                self._var_oa_url.get().strip(),
            )
        else:
            ai_session.configure_ollama(
                self._var_ol_url.get().strip(),
                self._var_ol_model.get().strip(),
            )

    def _load_ok(self):
        self._load_btn.config(state="normal",
                               text="  Перезагрузить модель  ")
        self._set_status(
            "Готово! Держи ← для Desktop-звука, → для Mic", "success")

    def _load_fail(self, msg: str):
        self._model_loaded = False
        self._ai_ready = False
        self._load_btn.config(state="normal",
                               text="  Загрузить модель  ")
        self._set_status(f"Ошибка: {msg}", "danger")
        mb.showerror("Ошибка загрузки", msg)

    def _on_send_manual(self):
        text = self._tr_text.get("1.0", "end").strip()
        if not text:
            self._set_status("Транскрипция пуста — нечего отправлять", "warning")
            return
        self._send_to_ai(text)

    def _clear_transcript(self):
        self._tr_text.delete("1.0", "end")
        self._char_count.config(text="0 симв.")

    def _clear_response(self):
        self._ai_text.config(state="normal")
        self._ai_text.delete("1.0", "end")
        self._ai_text.config(state="disabled")

    def _clear_history(self):
        ai_session.history.clear()
        self._set_status("История диалога очищена", "success")

    def _copy_response(self):
        text = self._ai_text.get("1.0", "end").strip()
        if text:
            self.clipboard_clear()
            self.clipboard_append(text)
            self._set_status("Скопировано в буфер обмена ✓", "success")

    def _browse_model(self):
        path = fd.askdirectory(title="Выбери папку с моделью VOSK")
        if path:
            self._var_model.set(path)

    def _test_connection(self):
        self._conn_label.config(text="Проверяю...", fg=C["warning"])
        try:
            self._configure_ai()
        except Exception as exc:
            err = str(exc)
            self._conn_label.config(text=f"Ошибка: {err}", fg=C["danger"])
            return

        def worker():
            ok, msg = ai_session.test_connection()
            color = C["success"] if ok else C["danger"]
            self.after(0, lambda: self._conn_label.config(
                text=msg, fg=color))

        threading.Thread(target=worker, daemon=True).start()

    def _save_settings(self):
        try:
            max_rec = int(self._var_maxrec.get())
        except ValueError:
            max_rec = 20

        # Сохранить выбранный loopback индекс
        loopback_idx = self._get_selected_loopback()
        if loopback_idx is None:
            loopback_idx = -1

        self._cfg.update({
            "position":            self._var_pos.get().strip(),
            "ai_backend":          self._var_backend.get(),
            "openai_api_key":      self._var_oa_key.get().strip(),
            "openai_model":        self._var_oa_model.get().strip(),
            "openai_base_url":     self._var_oa_url.get().strip(),
            "ollama_base_url":     self._var_ol_url.get().strip(),
            "ollama_model":        self._var_ol_model.get().strip(),
            "vosk_model_path":     self._var_model.get().strip(),
            "max_record_seconds":  max_rec,
            "loopback_device_index": loopback_idx,
            "system_prompt_template": self._prompt_text.get("1.0", "end").strip(),
        })
        cfg_module.save(self._cfg)
        self._save_lbl.config(text="Сохранено ✓", fg=C["success"])
        self.after(3000, lambda: self._save_lbl.config(text=""))

    # ─────────────── Статус ───────────────────────────────────────────────

    def _set_status(self, text: str, level: str = "normal"):
        clr = {
            "normal":  C["text_dim"],
            "success": C["success"],
            "warning": C["warning"],
            "danger":  C["danger"],
        }.get(level, C["text_dim"])
        self._dot.config(fg=clr)
        self._hdr_status.config(text=text, fg=clr)
        self._statusbar.config(text=text, fg=clr)

    # ─────────────── Закрытие ─────────────────────────────────────────────

    def _on_close(self):
        if hasattr(self, "_kb_listener"):
            try:
                self._kb_listener.stop()
            except Exception:
                pass
        transcriber.unload()
        self.destroy()


# ─────────────────────────── Запуск ───────────────────────────────────────

def launch():
    logging.basicConfig(level=logging.WARNING,
                        format="%(levelname)s %(name)s: %(message)s")
    App().mainloop()
