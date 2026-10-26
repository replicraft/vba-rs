' INV: vbal.5.6.9.3.1, op.neg
' SOURCE: spec ms-vbal 5.6.9.3.1 Unary - operator, negating the minimum Integer/Long held in a Variant widens to Long/Double
' EXPECT: Long 32768
' EXPECT: Double 2147483648
Public Sub Main()
    Dim v As Variant
    v = CInt(-32768)
    Debug.Print TypeName(-v) & " " & CStr(-v)
    v = CLng(-2147483647 - 1)
    Debug.Print TypeName(-v) & " " & CStr(-v)
End Sub
