' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 public enum name distinct from module names in the project
' STRICT_SPEC: true
' ERROR: load
Public Enum Other
    A1 = 1
End Enum
Public Sub Main()
End Sub
'#MODULE Other standard
Public Sub Q()
End Sub
