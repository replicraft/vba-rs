' INV: vbal.5.4.2.17, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.17 Exit Sub must be inside a subroutine declaration (not a property)
' ERROR: load
'#MODULE Cls class
Public Property Get P() As Long
    Exit Sub
End Property
'#MODULE Module1 standard
Public Sub Main()
End Sub
