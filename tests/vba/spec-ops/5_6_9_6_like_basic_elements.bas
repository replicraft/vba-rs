' INV: vbal.5.6.9.6, op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like pattern elements ? # * and literal characters
' EXPECT: TFTFTFTFTFTFFT
Private Function B(ByVal x As Boolean) As String
    If x Then B = "T" Else B = "F"
End Function
Public Sub Main()
    Dim r As String
    r = B("abc" Like "abc") & B("abc" Like "abd")
    r = r & B("abc" Like "a?c") & B("ac" Like "a?c")
    r = r & B("a5c" Like "a#c") & B("aXc" Like "a#c")
    r = r & B("abc" Like "a*") & B("abc" Like "b*")
    r = r & B("abc" Like "*c") & B("abc" Like "*b")
    r = r & B("abc" Like "*") & B("abc" Like "")
    r = r & B("" Like "?") & B("" Like "")
    Debug.Print r
End Sub
