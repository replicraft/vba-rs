' INV: vbal.5.6.13.2
' SOURCE: derived 5.6.13.2 the positional arguments left after the fixed parameters are queued into the ParamArray parameter, in order; the array is zero-based
' METHOD: by hand: G 1, 20, 30, 40 leaves three values: LBound 0, UBound 2, sum 90; G 1 leaves none, UBound -1
' EXPECT:  0 
' EXPECT:  2 
' EXPECT:  90 
' EXPECT: -1 
Private Sub G(a As Long, ParamArray rest() As Variant)
    Dim i As Long, t As Long
    For i = LBound(rest) To UBound(rest)
        t = t + rest(i)
    Next
    If UBound(rest) >= 0 Then Debug.Print LBound(rest)
    If UBound(rest) >= 0 Then Debug.Print UBound(rest)
    If UBound(rest) >= 0 Then Debug.Print t
    If UBound(rest) < 0 Then Debug.Print UBound(rest)
End Sub
Public Sub Main()
    G 1, 20, 30, 40
    G 1
End Sub
