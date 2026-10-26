' INV: op.pow
' SOURCE: spec ms-vbal 5.6.9.3.7 exponentiation operator
' EXPECT: 1024
' EXPECT: 1
' EXPECT: 0.5
' EXPECT: 3
Public Sub Main()
    Debug.Print CStr(2 ^ 10)
    Debug.Print CStr(2 ^ 0)
    Debug.Print CStr(2 ^ -1)
    Debug.Print CStr(9 ^ 0.5)
End Sub
