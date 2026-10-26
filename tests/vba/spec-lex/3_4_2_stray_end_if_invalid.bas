' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, an #End If without a matching #If does not conform to the grammar
' ERROR: load
Public Sub Main()
    Debug.Print "x"
#End If
End Sub
