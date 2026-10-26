' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 private UDT accessible only within the enclosing module
' ERROR: load
Public Sub Main()
    Dim p As Hidden
End Sub
'#MODULE Types standard
Private Type Hidden
    a As Long
End Type
