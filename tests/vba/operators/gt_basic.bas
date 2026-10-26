' INV: op.gt
' SOURCE: derived greater than
' EXPECT: True
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(2 > 1)
    Debug.Print CStr(2 > 2)
    Debug.Print CStr("b" > "a")
End Sub
