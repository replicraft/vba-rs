' INV: stmt.exit, stmt.property-get
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/exit-statement
' EXPECT:  1 
Property Get P() As Long
    P = 1
    Exit Property
    P = 2
End Property
Public Sub Main()
    Debug.Print P
End Sub
