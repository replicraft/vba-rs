' INV: op.div
' SOURCE: spec ms-vbal 5.6.9.3.5 division operator
' EXPECT: 3.5
' EXPECT: 2
' EXPECT: 0.25
' EXPECT: Double
Public Sub Main()
    Debug.Print CStr(7 / 2)
    Debug.Print CStr(6 / 3)
    Debug.Print CStr(1 / 4)
    Debug.Print CStr(TypeName(6 / 3))
End Sub
