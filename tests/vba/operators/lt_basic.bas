' INV: op.lt
' SOURCE: spec ms-vbal 5.6.9.5.3 less-than operator
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
