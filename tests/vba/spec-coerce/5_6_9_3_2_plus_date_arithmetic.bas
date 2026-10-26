' INV: vbal.5.6.9.3.2, op.add
' SOURCE: spec ms-vbal 5.6.9.3.2 + Operator, Date effective type: Double sum Let-coerced to Date
' XFAIL: wrong not implemented: Double promotion on Date/Single overflow with a Variant operand
' EXPECT: Date 7.5
' EXPECT: Date 7.5
' EXPECT: Date 5
Public Sub Main()
    Dim d As Date
    Dim r As Variant
    d = 2.5
    r = d + 5
    Debug.Print TypeName(r) & " " & CStr(CDbl(r))
    r = 5 + d
    Debug.Print TypeName(r) & " " & CStr(CDbl(r))
    Dim e As Date
    e = 2
    r = e + "3"
    Debug.Print TypeName(r) & " " & CStr(CDbl(r))
End Sub
