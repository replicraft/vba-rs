' INV: op.intdiv
' SOURCE: derived fractional portion truncated; operands rounded first (backslash-operator page)
' EXPECT: -3
' EXPECT: 4
Public Sub Main()
    Debug.Print CStr(-7 \ 2)
    Debug.Print CStr(7.6 \ 2)
End Sub
