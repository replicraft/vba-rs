' INV: vbal.5.6.1
' SOURCE: derived 5.6.1 a call of a function is a value expression, not a variable expression: it cannot be the target of an assignment
' METHOD: by construction
' ERROR: load
Private Function F(n As Long) As Long
    F = n
End Function
Public Sub Main()
    F(1) = 2
End Sub
