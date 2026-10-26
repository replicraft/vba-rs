' INV: expr.default-member
' SOURCE: derived an Attribute VB_UserMemId = 0 marks the default member
' EXPECT:  5 
' EXPECT:  6 
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
    w.Value = 5
    Debug.Print w
    Debug.Print w + 1
End Sub
