' INV: vbal.5.6.9.3.2, op.add
' SOURCE: spec ms-vbal 5.6.9.3.2 + Operator, String with a number operand is added numerically (effective type Double)
' EXPECT: 5.5
' EXPECT: Double
' EXPECT: 7
' EXPECT: 5.5
Public Sub Main()
    Dim s As String
    Dim n As Long
    s = "2.5"
    n = 3
    Debug.Print CStr(s + n)
    Debug.Print TypeName(s + n)
    Debug.Print CStr(3 + "4")
    Debug.Print CStr(n + s)
End Sub
