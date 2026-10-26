' INV: op.neg
' SOURCE: derived unary minus
' EXPECT: -5
' EXPECT: 5
' EXPECT: -5
Public Sub Main()
    Debug.Print CStr(-5)
    Debug.Print CStr(--5)
    Debug.Print CStr(-(2 + 3))
End Sub
