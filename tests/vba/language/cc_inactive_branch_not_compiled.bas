' INV: cc.if
' SOURCE: spec ms-vbal 3.4.2 excluded blocks are not part of the preprocessed module body
' EXPECT: ok
#If False Then
this is not valid VBA at all (((
#End If
Public Sub Main()
    Debug.Print "ok"
End Sub
