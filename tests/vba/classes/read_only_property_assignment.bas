' INV: stmt.property-get
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/property-get-statement
' ERROR: load
'#MODULE Box class
Public Property Get V() As Long
    V = 1
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Box
    b.V = 2
End Sub
