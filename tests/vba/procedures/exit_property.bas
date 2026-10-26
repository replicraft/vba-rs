' INV: stmt.exit, stmt.property-get
' SOURCE: spec ms-vbal 5.4.2.19 Exit Property statement
' EXPECT:  1 
Property Get P() As Long
    P = 1
    Exit Property
    P = 2
End Property
Public Sub Main()
    Debug.Print P
End Sub
