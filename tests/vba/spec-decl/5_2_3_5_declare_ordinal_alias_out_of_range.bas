' INV: vbal.5.2.3.5
' SOURCE: spec ms-vbal 5.2.3.5 ordinal alias must be in range 0 to 32,767
' XFAIL: wrong not implemented: validation of #ordinal aliases of Declare
' ERROR: load
Private Declare PtrSafe Function Bad Lib "somelib" Alias "#32768" () As Long
Public Sub Main()
End Sub
