' INV: vbal.5.3.1.7
' SOURCE: spec ms-vbal 5.3.1.7 Property Let and Get of one name have the same declared type
' ERROR: load
'#MODULE Cls class
Public Property Get P() As Long
End Property
Public Property Let P(ByVal v As String)
End Property
'#MODULE Module1 standard
Public Sub Main()
End Sub
