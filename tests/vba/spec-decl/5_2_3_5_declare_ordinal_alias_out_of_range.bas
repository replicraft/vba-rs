' INV: vbal.5.2.3.5
' SOURCE: spec ms-vbal 5.2.3.5 ordinal alias must be in range 0 to 32,767
' STRICT_SPEC: true
' ERROR: load
Private Declare PtrSafe Function Bad Lib "somelib" Alias "#32768" () As Long
Public Sub Main()
End Sub
