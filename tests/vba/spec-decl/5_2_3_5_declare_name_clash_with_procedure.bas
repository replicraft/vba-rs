' INV: vbal.5.2.3.5
' SOURCE: spec ms-vbal 5.2.3.5 external procedures are procedure declarations (names must be distinct)
' ERROR: load
Private Declare PtrSafe Function Dup Lib "somelib" () As Long
Private Function Dup() As Long
End Function
Public Sub Main()
End Sub
