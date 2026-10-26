' INV: expr.default-member, stmt.let
' SOURCE: derived a Let to an object whose default member is a read-only property does not compile
' ERROR: load
'#MODULE Wrap class
Private v As Long
Public Property Get Value() As Long
Attribute Value.VB_UserMemId = 0
    Value = v
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim w As New Wrap
    w = 5
End Sub
