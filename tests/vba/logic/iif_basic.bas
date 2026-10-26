' INV: func.iif
' SOURCE: derived IIf returns one of two parts
' EXPECT: 10
' EXPECT: b
Public Sub Main()
    Debug.Print CStr(IIf(1 < 2, 10, 20))
    Debug.Print CStr(IIf(False, "a", "b"))
End Sub
