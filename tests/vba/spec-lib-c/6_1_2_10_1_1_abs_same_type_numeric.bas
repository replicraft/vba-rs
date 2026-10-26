' INV: vbal.6.1.2.10.1.1, func.abs
' SOURCE: spec ms-vbal 6.1.2.10.1.1 Abs of a numeric value returns a value of the same type
' EXPECT: Integer 5
' EXPECT: Long 100000
' EXPECT: Single 2.5
' EXPECT: Double 7.25
' EXPECT: Currency 3.5
Public Sub Main()
    Dim a As Variant
    a = CInt(-5): Debug.Print TypeName(Abs(a)) & " " & CStr(Abs(a))
    a = CLng(-100000): Debug.Print TypeName(Abs(a)) & " " & CStr(Abs(a))
    a = CSng(-2.5): Debug.Print TypeName(Abs(a)) & " " & CStr(Abs(a))
    a = CDbl(-7.25): Debug.Print TypeName(Abs(a)) & " " & CStr(Abs(a))
    a = CCur(-3.5): Debug.Print TypeName(Abs(a)) & " " & CStr(Abs(a))
End Sub
