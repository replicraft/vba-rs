' INV: vbal.4.1
' SOURCE: derived 4.1 two Public procedures of one name in different modules make an unqualified use ambiguous (VBA reports "Ambiguous name detected")
' METHOD: by construction
' ERROR: load
'#MODULE A standard
Public Function Where() As String
    Where = "A"
End Function
'#MODULE B standard
Public Function Where() As String
    Where = "B"
End Function
'#MODULE Module1 standard
Public Sub Main()
    Debug.Print Where()
End Sub
