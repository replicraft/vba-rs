' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 public UDT name distinct from module names in the project
' XFAIL: wrong not implemented: uniqueness of public Type/Enum names across modules and modules/types
' ERROR: load
Public Type Other
    a As Long
End Type
Public Sub Main()
End Sub
'#MODULE Other standard
Public Sub Q()
End Sub
