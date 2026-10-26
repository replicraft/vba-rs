' INV: vbal.5.3.1
' SOURCE: spec ms-vbal 5.3.1 declared type of a function (suffix, As clause, implicit)
' EXPECT: Integer
' EXPECT: String
' EXPECT: Double
' EXPECT: Empty
Function A%()
    A = 1
End Function
Function B$()
    B = "x"
End Function
Function C() As Double
    C = 1
End Function
Function D()
End Function
Public Sub Main()
    Debug.Print TypeName(A())
    Debug.Print TypeName(B())
    Debug.Print TypeName(C())
    Debug.Print TypeName(D())
End Sub
