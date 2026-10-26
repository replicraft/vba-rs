' INV: vbal.5.6.10, decl.scope
' SOURCE: spec ms-vbal 5.6.10 two matches in the same tier make the simple name expression invalid
' ERROR: load
'#MODULE A standard
Public Function Dup() As Long
    Dup = 1
End Function
'#MODULE B standard
Public Function Dup() As Long
    Dup = 2
End Function
'#MODULE Module1 standard
Public Sub Main()
    Debug.Print Dup()
End Sub
