' INV: vbal.6.1.2.6.1.7, func.npv
' SOURCE: spec ms-vbal 6.1.2.6.1.7 NPV investment begins one period before the first cash flow
' EXPECT: one=100
' EXPECT: two=200
' EXPECT: type=Double
Public Sub Main()
    Dim a(0) As Double
    Dim b(1) As Double
    a(0) = 110
    b(0) = 110
    b(1) = 121
    Debug.Print "one=" & CStr(Round(NPV(0.1, a), 6))
    Debug.Print "two=" & CStr(Round(NPV(0.1, b), 6))
    Debug.Print "type=" & TypeName(NPV(0.1, a))
End Sub
