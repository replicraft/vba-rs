' INV: stmt.function
' SOURCE: spec ms-vbal 5.3.1 function result variable, 2.3 variable default values
' EXPECT:  0 []True
Function F() As Long
End Function
Function G() As String
End Function
Function H()
End Function
Public Sub Main()
    Debug.Print F(); "[" & G() & "]"; IsEmpty(H())
End Sub
