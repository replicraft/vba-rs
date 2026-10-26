' INV: op.eq
' SOURCE: derived equality
' EXPECT: True
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(1 = 1)
    Debug.Print CStr(1 = 2)
    Debug.Print CStr(1 = 1.0)
End Sub
