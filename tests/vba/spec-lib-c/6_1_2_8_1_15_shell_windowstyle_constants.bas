' INV: vbal.6.1.2.8.1.15, func.shell
' SOURCE: spec ms-vbal 6.1.2.8.1.15 Shell WindowStyle constants (VbAppWinStyle)
' EXPECT: 0 1 2 3 4 6
Public Sub Main()
    Debug.Print vbHide & " " & vbNormalFocus & " " & vbMinimizedFocus & " " & vbMaximizedFocus & " " & vbNormalNoFocus & " " & vbMinimizedNoFocus
End Sub
