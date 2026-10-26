' INV: op.or
' SOURCE: spec ms-vbal 5.6.9.8.3 Or operator
' EXPECT: 14
' EXPECT: 9
Public Sub Main()
    Debug.Print CStr(12 Or 10)
    Debug.Print CStr(8 Or 1)
End Sub
