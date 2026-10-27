' INV: vbal.5.6.1
' SOURCE: derived 5.6.1 a function name used without an argument list is classified as a value expression: it is called and yields its result
' METHOD: by hand: 40 + 2
' EXPECT:  42 
Private Function Answer() As Long
    Answer = 40
End Function
Public Sub Main()
    Debug.Print Answer + 2
End Sub
