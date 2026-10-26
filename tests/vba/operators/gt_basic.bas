' INV: op.gt
' SOURCE: spec ms-vbal 5.6.9.5.4 greater-than operator
' EXPECT: True
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(2 > 1)
    Debug.Print CStr(2 > 2)
    Debug.Print CStr("b" > "a")
End Sub
