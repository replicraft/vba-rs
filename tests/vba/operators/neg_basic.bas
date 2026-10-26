' INV: op.neg
' SOURCE: spec ms-vbal 5.6.9.3.1 unary minus operator
' EXPECT: -5
' EXPECT: 5
' EXPECT: -5
Public Sub Main()
    Debug.Print CStr(-5)
    Debug.Print CStr(--5)
    Debug.Print CStr(-(2 + 3))
End Sub
