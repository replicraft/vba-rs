' INV: vbal.5.4.3.8, stmt.let
' SOURCE: spec ms-vbal 5.4.3.8 property with only a Property Get of non-Variant type is invalid target
' ERROR: load
'#MODULE C1 class
Public Property Get P() As Long
    P = 1
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New C1
    c.P = 2
End Sub
