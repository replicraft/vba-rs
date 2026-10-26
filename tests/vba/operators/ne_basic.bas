' INV: op.ne
' SOURCE: spec ms-vbal 5.6.9.5.2 inequality operator
' EXPECT: True
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(1 <> 2)
    Debug.Print CStr(2 <> 2)
    Debug.Print CStr("a" <> "b")
End Sub
