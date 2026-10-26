' INV: vbal.5.6.16.4, stmt.redim
' SOURCE: spec ms-vbal 5.6.16.4 integer expression invalid when Let-coercion to Long is invalid (UDT)
' ERROR: load
Private Type P
    x As Long
End Type
Public Sub Main()
    Dim a() As Long
    Dim u As P
    ReDim a(0 To u)
End Sub
