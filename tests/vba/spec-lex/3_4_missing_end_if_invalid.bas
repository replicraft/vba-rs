' INV: vbal.3.4, cc.if
' SOURCE: spec ms-vbal 3.4 conditional compilation, a module whose lines do not conform to the cc-block grammar (#If without #End If) is not valid
' ERROR: load
Public Sub Main()
#If True Then
    Debug.Print "x"
End Sub
