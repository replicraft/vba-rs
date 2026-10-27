' INV: vbal.5.6.13.2
' SOURCE: derived 5.6.13.2 an optional parameter that receives no argument is left unfilled: for a Variant without a default, IsMissing reports it
' METHOD: by hand
' EXPECT: True
' EXPECT: False
Private Function M(Optional v As Variant) As Boolean
    M = IsMissing(v)
End Function
Public Sub Main()
    Debug.Print M()
    Debug.Print M(1)
End Sub
