' INV: expr.named-arg
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/function-statement
' EXPECT:  21 
' EXPECT:  34 
Function F(a As Long, b As Long) As Long
    F = a * 10 + b
End Function
Public Sub Main()
    Debug.Print F(b:=1, a:=2)
    Debug.Print F(3, b:=4)
End Sub
