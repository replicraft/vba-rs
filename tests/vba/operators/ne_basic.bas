' INV: op.ne
' SOURCE: derived inequality
' EXPECT: True
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(1 <> 2)
    Debug.Print CStr(2 <> 2)
    Debug.Print CStr("a" <> "b")
End Sub
