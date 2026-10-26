' INV: stmt.property-get, stmt.property-let
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/property-let-statement
' EXPECT:  8 
'#MODULE Box class
Private v As Long
Public Property Get Value() As Long
    Value = v
End Property
Public Property Let Value(ByVal n As Long)
    v = n * 2
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Box
    b.Value = 4
    Debug.Print b.Value
End Sub
