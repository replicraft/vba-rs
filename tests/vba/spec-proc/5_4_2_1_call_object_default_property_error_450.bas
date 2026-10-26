' INV: vbal.5.4.2.1, stmt.call
' SOURCE: spec ms-vbal 5.4.2.1 Call on an object whose default member is a property raises error 450
' XFAIL: wrong pending
' ERROR: 450
'#MODULE Wrap class
Public Property Get Value() As Long
Attribute Value.VB_UserMemId = 0
    Value = 1
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Set o = New Wrap
    Call o
End Sub
