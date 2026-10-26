' INV: op.lt
' SOURCE: derived less than
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(1 < 2)
    Debug.Print CStr(2 < 2)
    Debug.Print CStr("a" < "b")
    Debug.Print CStr("B" < "a")
End Sub
