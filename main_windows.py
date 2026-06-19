"""
Точка входа для PyInstaller.
Bootstrap не нужен — PyInstaller всё упаковал.
"""
import sys
import os

if getattr(sys, "frozen", False):
    os.chdir(os.path.dirname(sys.executable))

from assistant.app import launch
launch()
