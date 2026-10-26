' INV: stmt.property-get, stmt.property-let
' SOURCE: spec ms-vbal 5.4.3.8 Let-assignment through Property Let, 5.6.2.1 Property Get evaluation
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
