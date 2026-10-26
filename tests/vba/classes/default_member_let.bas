' INV: expr.default-member, stmt.let
' SOURCE: derived assigning to an object variable assigns its default property (Let)
' EXPECT:  5 
' XFAIL: unsupported 'w = 5' on an object whose default member has a Property Let is rejected with SEM007 (requires Set)
'#MODULE Wrap class
Private v As Long
Public Property Get Value() As Long
Attribute Value.VB_UserMemId = 0
    Value = v
End Property
Public Property Let Value(ByVal n As Long)
    v = n
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim w As New Wrap
    w = 5
    Debug.Print w
End Sub
