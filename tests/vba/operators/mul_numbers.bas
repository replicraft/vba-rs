' INV: op.mul
' SOURCE: spec ms-vbal 5.6.9.3.4 multiplication operator
' EXPECT: 42
' EXPECT: -12
' EXPECT: 2
Public Sub Main()
    Debug.Print CStr(6 * 7)
    Debug.Print CStr(-3 * 4)
    Debug.Print CStr(0.5 * 4)
End Sub
