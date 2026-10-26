' INV: vbal.4.2
' SOURCE: spec ms-vbal 4.2 modules, VB_Creatable MUST be False in a module of a source project
' ERROR: load
' STRICT_SPEC: true
'#MODULE Thing class
Attribute VB_Creatable = True
Public V As Long
'#MODULE Module1 standard
Public Sub Main()
End Sub
